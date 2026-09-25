//! TCP login gate with an explicit plaintext handshake and continuous encrypted streams.
//!
//! The session codec stays here because Apiimo currently exposes only the
//! plain frame codec and one-shot RC4 helper. One RC4 and one zlib state are
//! kept per direction after the RSA exchange, as required by the wire model.

use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

use apiimo_network::{
    decode_params, encode_params, EncryptKey, EncryptKeyString, EncryptToken, MsRequest, PbMessage,
};
use flate2::write::ZlibEncoder;
use flate2::{Compression, Decompress, FlushDecompress};
use rand::RngCore;
use rsa::pkcs1::{DecodeRsaPrivateKey, EncodeRsaPrivateKey, EncodeRsaPublicKey, LineEnding};
use rsa::{Oaep, RsaPrivateKey, RsaPublicKey};
use serde_json::json;
use sha1::Sha1;

const MAX_FRAME_LEN: u32 = 8 * 1024 * 1024;
const IO_POLL_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateFrame {
    pub method: u16,
    pub body: Vec<u8>,
}

pub struct GateKeyPair {
    pub private_key: RsaPrivateKey,
    pub public_key_pem: String,
}

/// Generate and persist a gate key once, using a private file mode on Unix.
/// The public PEM is the value the client needs in its own config.
pub fn load_or_generate_keypair(path: &Path) -> io::Result<GateKeyPair> {
    let private_key = if path.exists() {
        let pem = fs::read_to_string(path)?;
        RsaPrivateKey::from_pkcs1_pem(&pem)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?
    } else {
        let mut rng = rand::thread_rng();
        let generated = RsaPrivateKey::new(&mut rng, 2048)
            .map_err(|error| io::Error::other(error.to_string()))?;

        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }

        let pem = generated
            .to_pkcs1_pem(LineEnding::LF)
            .map_err(|error| io::Error::other(error.to_string()))?;
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(path) {
            Ok(mut file) => {
                file.write_all(pem.as_bytes())?;
                file.sync_all()?;
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                let pem = fs::read_to_string(path)?;
                return keypair_from_pem(&pem);
            }
            Err(error) => return Err(error),
        }
        generated
    };

    let public_key = RsaPublicKey::from(&private_key);
    let public_key_pem = public_key
        .to_pkcs1_pem(LineEnding::LF)
        .map_err(|error| io::Error::other(error.to_string()))?;
    Ok(GateKeyPair {
        private_key,
        public_key_pem,
    })
}

fn keypair_from_pem(pem: &str) -> io::Result<GateKeyPair> {
    let private_key = RsaPrivateKey::from_pkcs1_pem(pem)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    let public_key_pem = RsaPublicKey::from(&private_key)
        .to_pkcs1_pem(LineEnding::LF)
        .map_err(|error| io::Error::other(error.to_string()))?;
    Ok(GateKeyPair {
        private_key,
        public_key_pem,
    })
}

pub fn serve(
    address: &str,
    private_key: Arc<RsaPrivateKey>,
    shutdown: Arc<AtomicBool>,
) -> io::Result<()> {
    let listener = TcpListener::bind(address)?;
    listener.set_nonblocking(true)?;
    println!("serviimo-gate listening on {}", listener.local_addr()?);

    while shutdown.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _peer)) => {
                let private_key = Arc::clone(&private_key);
                thread::Builder::new()
                    .name("serviimo-gate-client".into())
                    .spawn(move || {
                        if let Err(error) = handle_connection(stream, &private_key) {
                            eprintln!("serviimo-gate connection ended: {}", error);
                        }
                    })?;
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(IO_POLL_INTERVAL);
            }
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn handle_connection(mut stream: TcpStream, private_key: &RsaPrivateKey) -> io::Result<()> {
    stream.set_nodelay(true)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;

    let first = read_plain_frame(&mut stream)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "missing method 1"))?;
    if first.method != 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "expected requestEncryptToken method 1",
        ));
    }

    let token = random_token();
    write_plain_frame(&mut stream, 19, &gate_info_body())?;
    write_plain_frame(
        &mut stream,
        10,
        &EncryptToken {
            token,
            compress_level: None,
            compress_type: None,
        }
        .encode_pb(),
    )?;

    let key_frame = read_plain_frame(&mut stream)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "missing method 2"))?;
    if key_frame.method != 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "expected confirmEncryptKey method 2",
        ));
    }
    let session_key = decrypt_key_exchange(&key_frame.body, private_key, token)?;

    // Acknowledgment bytes are the first bytes in the server-to-client
    // compressed and encrypted stream.
    let mut writer = EncryptedWriter::new(&session_key)?;
    let mut reader = EncryptedReader::new(&session_key)?;
    writer.write_frame(&mut stream, 11, &[])?;

    stream.set_read_timeout(Some(Duration::from_secs(15)))?;
    loop {
        match reader.read_frame(&mut stream) {
            Ok(frame) => {
                if frame.method == 7 {
                    match MsRequest::decode_pb(&frame.body) {
                        Ok(request) => match verify_sdk_response(&request) {
                            Ok(response) => writer.write_frame(&mut stream, 8, &response)?,
                            Err(error) => {
                                eprintln!("serviimo-gate rejected microservice request: {}", error);
                            }
                        },
                        Err(error) => {
                            eprintln!(
                                "serviimo-gate could not decode microservice request: {}",
                                error
                            );
                        }
                    }
                } else if frame.method == 6 {
                    // Heartbeats are accepted so idle clients do not terminate the RPC session.
                } else {
                    eprintln!("serviimo-gate ignored method {}", frame.method);
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                return Ok(());
            }
            Err(error) => return Err(error),
        }
    }
}

fn gate_info_body() -> Vec<u8> {
    // Apiimo's live probe records two 12-byte values on method 19, but the
    // shipped schema does not identify this push. Keep valid opaque fields.
    let mut message = PbMessage::new();
    message.set_bytes(1, vec![0; 12]);
    message.set_bytes(2, vec![0; 12]);
    message.encode()
}

fn random_token() -> i64 {
    let mut bytes = [0u8; 8];
    rand::thread_rng().fill_bytes(&mut bytes);
    (u64::from_le_bytes(bytes) & i64::MAX as u64) as i64
}

fn decrypt_key_exchange(
    body: &[u8],
    private_key: &RsaPrivateKey,
    expected_token: i64,
) -> io::Result<Vec<u8>> {
    let wrapped = EncryptKeyString::decode_pb(body)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let plaintext = private_key
        .decrypt(Oaep::new::<Sha1>(), &wrapped.key_string)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    let material = EncryptKey::decode_pb(&plaintext)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    if material.token != expected_token {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "encrypt token mismatch",
        ));
    }
    if material.key.len() != 20 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "session key must contain 20 bytes",
        ));
    }
    Ok(material.key)
}

/// Validate the exact verifySdk call shape and return an encoded MsResponse.
fn verify_sdk_response(request: &MsRequest) -> io::Result<Vec<u8>> {
    if request.rid != Some(1)
        || request.service != "LoginService"
        || request.method != "verifySdk"
        || request.caller_id.as_deref() != Some("")
        || request.context.as_deref() != Some(&[0x90][..])
        || request.options.as_deref() != Some(&[0x90][..])
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unexpected verifySdk envelope",
        ));
    }

    let params = request
        .params
        .as_deref()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing verifySdk params"))?;
    let decoded =
        decode_params(params).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let values = decoded.as_array().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "verifySdk params must be an array",
        )
    })?;
    if values.len() != 2 || !values[1].is_boolean() || values[1].as_bool() != Some(false) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "verifySdk params must contain ticket and false",
        ));
    }
    if params.first() != Some(&0x92) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "verifySdk params must use MessagePack fixarray2",
        ));
    }
    let ticket = decode_verify_ticket(params)?;
    if values[0].as_str() != Some(ticket.as_str()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid verifySdk ticket",
        ));
    }

    let mut random = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut random);
    let token = random
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect::<String>();
    let payload = json!({
        "status": 1,
        "Token": token,
        "GlobalGateSessionId": "serviimo-session",
        "GlobalGateId": "serviimo-world",
        "ServerId": "1",
        "RoleInfo": {
            "players": [{
                "roleId": "302001142689",
                "roleName": "Serviimo",
                "level": 1
            }]
        },
    });

    let mut response = PbMessage::new();
    response.set_varint(1, request.rid.unwrap_or(1));
    response.set_bytes(2, encode_params(&payload));
    Ok(response.encode())
}

fn decode_verify_ticket(params: &[u8]) -> io::Result<String> {
    if params.len() < 5 || params[0] != 0x92 || params[1] != 0xda {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "verifySdk ticket must use fixarray2 and str16",
        ));
    }
    let ticket_len = u16::from_be_bytes([params[2], params[3]]) as usize;
    let ticket_end = 4 + ticket_len;
    if params.len() != ticket_end + 1 || params[ticket_end] != 0xc2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "verifySdk params must end with false",
        ));
    }
    String::from_utf8(params[4..ticket_end].to_vec())
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn encode_frame(method: u16, body: &[u8]) -> io::Result<Vec<u8>> {
    let total = body
        .len()
        .checked_add(2)
        .filter(|length| *length <= MAX_FRAME_LEN as usize)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "frame is too large"))?;
    let mut bytes = Vec::with_capacity(4 + total);
    bytes.extend_from_slice(&(total as u32).to_le_bytes());
    bytes.extend_from_slice(&method.to_le_bytes());
    bytes.extend_from_slice(body);
    Ok(bytes)
}

fn write_plain_frame<W: Write>(writer: &mut W, method: u16, body: &[u8]) -> io::Result<()> {
    writer.write_all(&encode_frame(method, body)?)
}

fn read_plain_frame<R: Read>(reader: &mut R) -> io::Result<Option<GateFrame>> {
    let mut length = [0u8; 4];
    match reader.read(&mut length[..1])? {
        0 => return Ok(None),
        1 => {}
        _ => unreachable!(),
    }
    reader.read_exact(&mut length[1..])?;
    let total = u32::from_le_bytes(length);
    if total < 2 || total > MAX_FRAME_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid frame length",
        ));
    }
    let mut payload = vec![0u8; total as usize];
    reader.read_exact(&mut payload)?;
    Ok(Some(GateFrame {
        method: u16::from_le_bytes([payload[0], payload[1]]),
        body: payload[2..].to_vec(),
    }))
}

struct EncryptedWriter {
    cipher: Rc4Stream,
    compressor: ZlibEncoder<Vec<u8>>,
}

impl EncryptedWriter {
    fn new(key: &[u8]) -> io::Result<Self> {
        Ok(Self {
            cipher: Rc4Stream::new(key)?,
            compressor: ZlibEncoder::new(Vec::new(), Compression::none()),
        })
    }

    fn encode_frame(&mut self, method: u16, body: &[u8]) -> io::Result<Vec<u8>> {
        let frame = encode_frame(method, body)?;
        self.compressor.write_all(&frame)?;
        self.compressor.flush()?;
        let mut compressed = std::mem::take(self.compressor.get_mut());
        self.cipher.apply(&mut compressed);
        Ok(compressed)
    }

    fn write_frame<W: Write>(
        &mut self,
        writer: &mut W,
        method: u16,
        body: &[u8],
    ) -> io::Result<()> {
        writer.write_all(&self.encode_frame(method, body)?)
    }
}

struct EncryptedReader {
    cipher: Rc4Stream,
    decompressor: Decompress,
    pending: Vec<u8>,
}

impl EncryptedReader {
    fn new(key: &[u8]) -> io::Result<Self> {
        Ok(Self {
            cipher: Rc4Stream::new(key)?,
            decompressor: Decompress::new(true),
            pending: Vec::new(),
        })
    }

    fn push_ciphertext(&mut self, ciphertext: &[u8]) -> io::Result<()> {
        let mut decrypted = ciphertext.to_vec();
        self.cipher.apply(&mut decrypted);
        let mut input_offset = 0usize;
        loop {
            let mut output = [0u8; 4096];
            let before_in = self.decompressor.total_in();
            let before_out = self.decompressor.total_out();
            self.decompressor
                .decompress(
                    &decrypted[input_offset..],
                    &mut output,
                    FlushDecompress::Sync,
                )
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
            let consumed = (self.decompressor.total_in() - before_in) as usize;
            let produced = (self.decompressor.total_out() - before_out) as usize;
            input_offset += consumed;
            self.pending.extend_from_slice(&output[..produced]);
            if input_offset >= decrypted.len() && produced < output.len() {
                break;
            }
            if consumed == 0 && produced == 0 {
                break;
            }
        }
        Ok(())
    }

    fn pop_frame(&mut self) -> io::Result<Option<GateFrame>> {
        if self.pending.len() < 4 {
            return Ok(None);
        }
        let total = u32::from_le_bytes(self.pending[..4].try_into().unwrap());
        if total < 2 || total > MAX_FRAME_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid encrypted frame length",
            ));
        }
        let frame_len = 4usize + total as usize;
        if self.pending.len() < frame_len {
            return Ok(None);
        }
        let payload = self.pending[4..frame_len].to_vec();
        self.pending.drain(..frame_len);
        Ok(Some(GateFrame {
            method: u16::from_le_bytes([payload[0], payload[1]]),
            body: payload[2..].to_vec(),
        }))
    }

    fn read_frame(&mut self, stream: &mut TcpStream) -> io::Result<GateFrame> {
        loop {
            if let Some(frame) = self.pop_frame()? {
                return Ok(frame);
            }
            let mut ciphertext = [0u8; 4096];
            let count = stream.read(&mut ciphertext)?;
            if count == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "encrypted stream closed",
                ));
            }
            self.push_ciphertext(&ciphertext[..count])?;
        }
    }
}

struct Rc4Stream {
    state: [u8; 256],
    index_i: u8,
    index_j: u8,
}

impl Rc4Stream {
    fn new(key: &[u8]) -> io::Result<Self> {
        if key.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "RC4 key must not be empty",
            ));
        }
        let mut state = [0u8; 256];
        for (index, value) in state.iter_mut().enumerate() {
            *value = index as u8;
        }
        let mut j = 0usize;
        for i in 0..256 {
            j = (j + state[i] as usize + key[i % key.len()] as usize) & 0xff;
            state.swap(i, j);
        }
        Ok(Self {
            state,
            index_i: 0,
            index_j: 0,
        })
    }

    fn apply(&mut self, bytes: &mut [u8]) {
        for byte in bytes {
            self.index_i = self.index_i.wrapping_add(1);
            self.index_j = self.index_j.wrapping_add(self.state[self.index_i as usize]);
            self.state
                .swap(self.index_i as usize, self.index_j as usize);
            let key_byte = self.state[self.state[self.index_i as usize]
                .wrapping_add(self.state[self.index_j as usize])
                as usize];
            *byte ^= key_byte;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apiimo_network::{decode_params, EncryptKey, EncryptKeyString, MsRequest};
    use rand::rngs::OsRng;
    use rsa::{Oaep, RsaPrivateKey};
    use serde_json::Value;

    #[test]
    fn framing_codec_round_trip() {
        let encoded = encode_frame(1, &[]).unwrap();
        assert_eq!(encoded, vec![2, 0, 0, 0, 1, 0]);
        let mut input = io::Cursor::new(encoded);
        let decoded = read_plain_frame(&mut input).unwrap().unwrap();
        assert_eq!(
            decoded,
            GateFrame {
                method: 1,
                body: Vec::new()
            }
        );
    }

    #[test]
    fn client_shaped_handshake_and_continuous_encrypted_round_trip() {
        let private_key = RsaPrivateKey::new(&mut OsRng, 2048).unwrap();
        let public_key = RsaPublicKey::from(&private_key);
        let token = 0x1020_3040i64;

        // Client sends method 1; gate replies with the observed 19 then 10 order.
        assert_eq!(
            read_plain_frame(&mut io::Cursor::new(encode_frame(1, &[]).unwrap()))
                .unwrap()
                .unwrap()
                .method,
            1
        );
        let server_handshake_bytes = [
            encode_frame(19, &gate_info_body()).unwrap(),
            encode_frame(
                10,
                &EncryptToken {
                    token,
                    compress_level: None,
                    compress_type: None,
                }
                .encode_pb(),
            )
            .unwrap(),
        ]
        .concat();
        let mut server_handshake = io::Cursor::new(server_handshake_bytes);
        let info = read_plain_frame(&mut server_handshake).unwrap().unwrap();
        let token_frame = read_plain_frame(&mut server_handshake).unwrap().unwrap();
        assert_eq!(info.method, 19);
        assert_eq!(
            EncryptToken::decode_pb(&token_frame.body).unwrap().token,
            token
        );

        let session_key: Vec<u8> = (0..20).map(|value| value as u8).collect();
        let plaintext = EncryptKey {
            head_padding: vec![1, 2, 3, 4, 5],
            key: session_key.clone(),
            token,
            tail_padding: vec![9; 13],
        }
        .encode_pb();
        let ciphertext = public_key
            .encrypt(&mut OsRng, Oaep::new::<Sha1>(), &plaintext)
            .unwrap();
        let method2_body = EncryptKeyString {
            key_string: ciphertext,
        }
        .encode_pb();
        let method2_wire = encode_frame(2, &method2_body).unwrap();
        let method2_frame = read_plain_frame(&mut io::Cursor::new(method2_wire))
            .unwrap()
            .unwrap();
        assert_eq!(method2_frame.method, 2);
        let recovered_key = decrypt_key_exchange(&method2_frame.body, &private_key, token).unwrap();
        assert_eq!(recovered_key.len(), 20);
        assert_eq!(recovered_key, session_key);

        // Method 11 is inside the first server-to-client encrypted stream frame.
        let mut gate_to_client = EncryptedWriter::new(&recovered_key).unwrap();
        let mut client_from_gate = EncryptedReader::new(&recovered_key).unwrap();
        let ack = gate_to_client.encode_frame(11, &[]).unwrap();
        client_from_gate.push_ciphertext(&ack).unwrap();
        assert_eq!(client_from_gate.pop_frame().unwrap().unwrap().method, 11);

        // Method 7 and method 8 share one continuous stream in each direction.
        let ticket = "personal-use-ticket";
        let request = MsRequest {
            rid: Some(1),
            service: "LoginService".into(),
            method: "verifySdk".into(),
            params: Some(encode_ticket_params(ticket)),
            caller_id: Some(String::new()),
            context: Some(vec![0x90]),
            options: Some(vec![0x90]),
            ..Default::default()
        };
        let mut client_to_gate = EncryptedWriter::new(&recovered_key).unwrap();
        let mut gate_from_client = EncryptedReader::new(&recovered_key).unwrap();
        let request_wire = client_to_gate
            .encode_frame(7, &request.encode_pb())
            .unwrap();
        for chunk in request_wire.chunks(3) {
            gate_from_client.push_ciphertext(chunk).unwrap();
        }
        let request_frame = gate_from_client.pop_frame().unwrap().unwrap();
        assert_eq!(request_frame.method, 7);
        let decoded_request = MsRequest::decode_pb(&request_frame.body).unwrap();
        assert_eq!(
            decoded_request.params.as_deref().unwrap()[0],
            0x92,
            "params begin with MessagePack fixarray2"
        );

        let response = verify_sdk_response(&decoded_request).unwrap();
        let response_wire = gate_to_client.encode_frame(8, &response).unwrap();
        client_from_gate.push_ciphertext(&response_wire).unwrap();
        let response_frame = client_from_gate.pop_frame().unwrap().unwrap();
        assert_eq!(response_frame.method, 8);
        let response_message = apiimo_network::pb_decode(&response_frame.body).unwrap();
        let payload: Value = decode_params(response_message.bytes(2).unwrap()).unwrap();
        assert_eq!(payload["status"], 1);
        assert_eq!(payload["Token"].as_str().unwrap().len(), 32);
        assert_eq!(payload["RoleInfo"]["players"][0]["roleId"], "302001142689");
    }

    fn encode_ticket_params(ticket: &str) -> Vec<u8> {
        let bytes = ticket.as_bytes();
        let mut params = vec![0x92, 0xda];
        params.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
        params.extend_from_slice(bytes);
        params.push(0xc2);
        params
    }
}

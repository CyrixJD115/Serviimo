//! Single-process runner for the TCP and HTTP service threads.

mod config;

use std::error::Error;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

use config::{ensure_parent, Config};

fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let Some(config) = Config::from_args(&arguments)? else {
        return Ok(());
    };

    ensure_parent(&config.save_path)?;
    let _store = serviimo_store::Store::open(&config.save_path)?;
    let keypair = serviimo_gate::load_or_generate_keypair(&config.rsa_keypair_path)?;
    config.write_aniisignal_config(&keypair.public_key_pem)?;

    println!("serviimo starting");
    println!("  gate bind: {}", config.bind_addresses.gate);
    println!("  cdn bind: {}", config.bind_addresses.cdn);
    println!("  office bind: {}", config.bind_addresses.office);
    println!(
        "  world gate bind (reserved): {}",
        config.bind_addresses.world
    );
    println!(
        "  advertised gate address: {}",
        config.published_gate_address()
    );
    println!("  RSA keypair path: {}", config.rsa_keypair_path.display());
    println!("  save path: {}", config.save_path.display());
    println!(
        "  CDN document root: {}",
        config.cdn_document_root.display()
    );
    println!(
        "  AniiSignal config: {}",
        config.aniisignal_config_path.display()
    );
    println!("  gate public key PEM:");
    print!("{}", keypair.public_key_pem);

    let shutdown = Arc::new(AtomicBool::new(true));
    let signal_shutdown = Arc::clone(&shutdown);
    ctrlc::set_handler(move || {
        signal_shutdown.store(false, Ordering::SeqCst);
    })?;

    let mut tasks = Vec::new();

    {
        let shutdown = Arc::clone(&shutdown);
        let address = config.bind_addresses.gate.clone();
        let private_key = Arc::new(keypair.private_key);
        tasks.push(
            thread::Builder::new()
                .name("serviimo-gate".into())
                .spawn(move || {
                    if let Err(error) =
                        serviimo_gate::serve(&address, private_key, Arc::clone(&shutdown))
                    {
                        eprintln!("serviimo-gate failed: {}", error);
                        shutdown.store(false, Ordering::SeqCst);
                    }
                })?,
        );
    }

    {
        let shutdown = Arc::clone(&shutdown);
        let address = config.bind_addresses.cdn.clone();
        let document_root = config.cdn_document_root.clone();
        let gate_address = config.published_gate_address().to_string();
        tasks.push(
            thread::Builder::new()
                .name("serviimo-cdn".into())
                .spawn(move || {
                    if let Err(error) = serviimo_cdn::serve(
                        &address,
                        &document_root,
                        &gate_address,
                        Arc::clone(&shutdown),
                    ) {
                        eprintln!("serviimo-cdn failed: {}", error);
                        shutdown.store(false, Ordering::SeqCst);
                    }
                })?,
        );
    }

    {
        let shutdown = Arc::clone(&shutdown);
        let address = config.bind_addresses.office.clone();
        tasks.push(
            thread::Builder::new()
                .name("serviimo-office".into())
                .spawn(move || {
                    if let Err(error) = serviimo_office::serve(&address, Arc::clone(&shutdown)) {
                        eprintln!("serviimo-office failed: {}", error);
                        shutdown.store(false, Ordering::SeqCst);
                    }
                })?,
        );
    }

    while shutdown.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(100));
    }
    for task in tasks {
        let _ = task.join();
    }
    println!("serviimo stopped");
    Ok(())
}

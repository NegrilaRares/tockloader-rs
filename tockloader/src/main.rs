// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OXIDOS AUTOMOTIVE 2024.

mod board_attributes;
mod board_settings;
mod cli;
mod errors;
mod interfaces;
mod kernel_attributes;

use board_attributes::{get_all_attributes, get_bootloader_version};
use board_settings::BoardSettings;
use clap::ArgMatches;
use cli::make_cli;
use errors::TockloaderError;
use kernel_attributes::kernel_attributes;

use inquire::Select;
use probe_rs::probe::list::Lister;
use probe_rs::{Core, MemoryInterface, Permissions};
use tbf_parser::parse::*;
use tbf_parser::types::*;

#[tokio::main]
async fn main() -> Result<(), TockloaderError> {
    let result = run().await;
    if let Err(e) = &result {
        eprintln!("\n{}", e);
    }
    result
}

async fn run() -> Result<(), TockloaderError> {
    let matches = make_cli().get_matches();

    if matches.get_flag("debug") {
        println!("Debug mode enabled");
    }

    match matches.subcommand() {
        Some(("listen", _sub_matches)) => {
            match tock_process_console::run().await {
                Ok(()) => {}
                Err(_) => {
                    print!("cli bricked!")
                }
            };
        }
        Some(("list", sub_matches)) => {
            list_probes(sub_matches).await?;
        }
      
        Some(("install", _sub_matches)) => {}
        Some(("info", sub_matches)) => {
            info_probe(sub_matches).await;
        }

        // If only the "--debug" flag is set, then this branch is executed
        // Or, more likely at this stage, a subcommand hasn't been implemented yet.
        _ => {
            println!("Could not run the provided subcommand.");
            _ = make_cli().print_help();
        }
    }

    Ok(())
}

async fn list_probes(sub_matches: &ArgMatches) -> Result<(), TockloaderError> {
    let lister = Lister::new();
    let probes = lister.list_all();
    println!("Probes: {:?}\n", probes);

    let ans = Select::new("Which probe do you want to use?", probes).prompt();

    match ans {
        Ok(choice) => {
            let probe = choice.open().unwrap();

            let chip = sub_matches.get_one::<String>("chip").unwrap();
            let board = sub_matches.get_one::<String>("board").unwrap();

            let board_settings = BoardSettings::new(board.clone(), chip.clone());

            let mut session = probe
                .attach(board_settings.chip, Permissions::default())
                .unwrap();

            println!("Session target: {:?}\n", session.target());
            println!("Session interfaces: {:?}\n", session.architecture());
            println!("Session core: {:?}\n", session.list_cores());

            let core_index = sub_matches.get_one::<usize>("core").unwrap();

            let mut core = session.core(*core_index).unwrap();

            let mut address = board_settings.start_address;

            // Jump through the linked list of apps
            loop {
                // Read a block of 200 8-bit words
                let mut buff = vec![0u8; 200];
                match core.read(address, &mut buff) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Error reading memory: {:?}", e);
                        break;
                    }
                }

                let (ver, header_len, whole_len) =
                    match parse_tbf_header_lengths(&buff[0..8].try_into().unwrap()) {
                        Ok((ver, header_len, whole_len)) if header_len != 0 => {
                            println!("Version: {:?}\n", ver);
                            println!("Header length: {:?}\n", header_len);
                            println!("Whole length: {:?}\n", whole_len);
                            (ver, header_len, whole_len)
                        }
                        _ => break, // No more apps
                    };

                let header = parse_tbf_header(&buff[0..header_len as usize], ver);
                match header {
                    Ok(header) => {
                        println!("Enabled: {:?}\n", header.enabled());
                        println!(
                            "Minimum App Ram Size: {:?}\n",
                            header.get_minimum_app_ram_size()
                        );
                        println!(
                            "Init function offset: {:?}\n",
                            header.get_init_function_offset()
                        );
                        println!("Protected size: {:?}\n", header.get_protected_size());
                        println!(
                            "Package name: {:?}\n",
                            header.get_package_name().unwrap_or_default()
                        );
                        println!(
                            "Kernel version: {:?}\n",
                            header.get_kernel_version().unwrap_or_default()
                        );
                    }
                    // TODO(MicuAna): refactor when reworking errors
                    Err(TbfParseError::ChecksumMismatch(
                        provided_checksum,
                        calculated_checksum,
                    )) => {
                        println!(
                            "Checksum mismatch: provided = {}, calculated = {}",
                            provided_checksum, calculated_checksum
                        );
                        break;
                    }
                    Err(e) => {
                        println!("Failed to parse TBF header: {:?}", e);
                        break;
                    }
                }
                address += whole_len as u64;
            }
        }
        Err(err) => println!("{}", err),
    }

    Ok(())
}

async fn info_probe(sub_matches: &ArgMatches) {
    println!("entered");
    let lister = Lister::new();
    let probes = lister.list_all();

    let ans = Select::new("Which probe do you want to use?", probes).prompt();
    match ans {
        Ok(choice) => {
            let probe = choice.open().unwrap();

            let chip = sub_matches.get_one::<String>("chip").unwrap();
            let board = sub_matches.get_one::<String>("board").unwrap();

            let board_settings = BoardSettings::new(board.clone(), chip.clone());

            let mut session = probe
                .attach(board_settings.chip, Permissions::default())
                .unwrap();

            let core_index = sub_matches.get_one::<usize>("core").unwrap();

            let mut core = session.core(*core_index).unwrap();

            let bootloader_version = get_bootloader_version(&mut core);

            let mut attributes = get_all_attributes(&mut core);


            println!(
                "Bootloader Version: {}                [0x40E]",
                bootloader_version
            );
            println!("Kernel Attributes");

            kernel_attributes(&mut core, &mut attributes);
        }
        Err(err) => println!("While picking probe:{}", err),
    }
}


fn info_app_list(mut board_core: Core, board_settings: BoardSettings) {
    let mut address = board_settings.start_address;
    loop {
        // Read a block of 200 8-bit words
        let mut buff = vec![0u8; 200];
        match board_core.read(address, &mut buff) {
            Ok(_) => {}
            Err(e) => {
                println!("Error reading memory: {:?}", e);
                break;
            }
        }

        let (ver, header_len, whole_len) =
            match parse_tbf_header_lengths(&buff[0..8].try_into().unwrap()) {
                Ok((ver, header_len, whole_len)) if header_len != 0 => {
                    println!("Version: {:?}\n", ver);
                    println!("Header length: {:?}\n", header_len);
                    println!("Whole length: {:?}\n", whole_len);
                    (ver, header_len, whole_len)
                }
                _ => break, // No more apps
            };

        let header = parse_tbf_header(&buff[0..header_len as usize], ver);
        match header {
            Ok(header) => {
                println!("Enabled: {:?}\n", header.enabled());
                println!(
                    "Minimum App Ram Size: {:?}\n",
                    header.get_minimum_app_ram_size()
                );
                println!(
                    "Init function offset: {:?}\n",
                    header.get_init_function_offset()
                );
                println!("Protected size: {:?}\n", header.get_protected_size());
                println!(
                    "Package name: {:?}\n",
                    header.get_package_name().unwrap_or_default()
                );
                println!(
                    "Kernel version: {:?}\n",
                    header.get_kernel_version().unwrap_or_default()
                );
            }
            // TODO(MicuAna): refactor when reworking errors
            Err(TbfParseError::ChecksumMismatch(provided_checksum, calculated_checksum)) => {
                println!(
                    "Checksum mismatch: provided = {}, calculated = {}",
                    provided_checksum, calculated_checksum
                );
                break;
            }
            Err(e) => {
                println!("Failed to parse TBF header: {:?}", e);
                break;
            }
        }
        address += whole_len as u64;
    }
}
// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OXIDOS AUTOMOTIVE 2024.

use core::str;
use std::collections::HashMap;

use probe_rs::{Core, MemoryInterface};

pub fn kernel_attributes(board_core: &mut Core, attributes: &mut HashMap<String, String>) {
    let address_apps = i32::from_str_radix(
        attributes
            .get("appaddr")
            .expect("Error encountered while getting the appaddr for kernel attributes.")
            .trim_start_matches("0x"),
        16,
    )
    .unwrap();
    let mut kernel_attr_binary = [0u8; 100];
    let _ = board_core.read(
        (address_apps - 100).try_into().unwrap(),
        &mut kernel_attr_binary,
    );
    // println!("{:?}", kernel_attr_binary);

    let sentinel = bytes_to_string(&kernel_attr_binary[96..100]);
    let version = &kernel_attr_binary[92..96]; //str::from_utf8(&kernel_attr_binary[92..96]).expect("Not UTF-8.");

    let mut version_formated: u16 = 0;
    let mut prefix = true;
    for i in 0..version.len() {
        if version[i] == 0 && prefix == true {
        } else {
            prefix = false;
            version_formated = version_formated * 10 + version[i] as u16;
        }
    }

    let app_memory = &kernel_attr_binary[88..92];

    println!("app_Mem: {:?}", app_memory);

    println!("Kernel Attributes");
    println!("  Sentinel: {:?}", sentinel);
    println!("  Version: {:?}", version_formated);
    println!("KATLV: APP Memory (0x101)");
    // println!("  app_memory_start: {:?}",);
    // println!("  app_memory_len: {:?}",);
}

// TODO(RARES): will have to use this in board attributes too where needed to debload some of the code
pub fn bytes_to_string(raw: &[u8]) -> String {
    let decoder = utf8_decode::Decoder::new(raw.iter().cloned());

    let mut string = String::new();
    for n in decoder {
        string.push(n.expect("Error getting key for attributes."));
    }
    string
}

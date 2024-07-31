// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OXIDOS AUTOMOTIVE 2024.

use std::collections::HashMap;

use byteorder::{ByteOrder, LittleEndian};
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

    let sentinel = bytes_to_string(&kernel_attr_binary[96..100]);
    let version = LittleEndian::read_uint(&kernel_attr_binary[95..96], 1);

    let app_memory_len = LittleEndian::read_u32(&kernel_attr_binary[84..92]);
    let app_memory_start = LittleEndian::read_u32(&kernel_attr_binary[80..84]);

    let kernel_binary_start = LittleEndian::read_u32(&kernel_attr_binary[68..72]);
    let kernel_binary_len = LittleEndian::read_u32(&kernel_attr_binary[72..76]);

    println!("Kernel Attributes");
    println!("  Sentinel: {:?}", sentinel);
    println!("  Version: {}", version);
    println!("KATLV: APP Memory");
    println!("  app_memory_start: {:?}", app_memory_start);
    println!("  app_memory_len: {:?}", app_memory_len);
    println!("KATLV: Kernel Binary");
    println!("  kernel_binary_start: {:?}", kernel_binary_start);
    println!("  kernel_binary_len: {:?}", kernel_binary_len);
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

// pub fn get_kernel_version(board_core: &mut Core) -> u8 {
//     let addr = 0x3FFFC;
//     let mut version = [0u8; 1];
//     let _ = board_core.read((addr).try_into().unwrap(), &mut version);
//     version[0]
// }

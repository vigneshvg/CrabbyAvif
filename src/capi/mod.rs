// Copyright 2024 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// This module contains functions and types that will be exposed in the C API header file.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]
#![allow(dead_code)]

mod decoder;
#[cfg(feature = "encoder")]
mod encoder;
mod gainmap;
mod image;
mod io;
mod reformat;
mod types;

#[macro_export]
macro_rules! deref_const {
    ($ptr:expr) => {{
        // The extra curly braces here is necessary to make this whole macro into a single
        // expression.
        assert!(!$ptr.is_null());
        unsafe { &*($ptr) }
    }};
}

#[macro_export]
macro_rules! deref_mut {
    ($ptr:expr) => {{
        // The extra curly braces here is necessary to make this whole macro into a single
        // expression.
        assert!(!$ptr.is_null());
        unsafe { &mut *($ptr) }
    }};
}

#[macro_export]
macro_rules! check_pointer {
    ($ptr:expr) => {
        if $ptr.is_null() {
            return avifResult::InvalidArgument;
        }
    };
}

#[macro_export]
macro_rules! check_pointer_or_return {
    ($ptr:expr) => {
        if $ptr.is_null() {
            return;
        }
    };
}

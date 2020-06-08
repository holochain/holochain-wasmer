use holochain_wasmer_guest::*;

holochain_wasmer_guest::holochain_externs!();

macro_rules! _s {
    ( $t:tt; $empty:expr; ) => {
        paste::item! {
            // @TODO - this is dangerous at the moment
            // if the guest fails to call host_args!() then the host leaks the input indefinitely
            //
            // #[no_mangle]
            // ignore the input completely and return empty data
            // pub extern "C" fn [< $t:lower _input_ignored_empty_ret >](ptr: RemotePtr) -> RemotePtr {
            //     ret!(
            //         paste::expr! {
            //             test_common::[< $t:camel Type >]::from($empty)
            //         }
            //     );
            // }

            #[no_mangle]
            /// load the input args and do nothing with it
            pub extern "C" fn [< $t:lower _input_args_empty_ret >](ptr: RemotePtr) -> RemotePtr {
                paste::expr! {
                    let _: test_common::[< $t:camel Type >] = host_args!(ptr);
                }
                ret!(
                    paste::expr! {
                        test_common::[< $t:camel Type >]::from($empty)
                    }
                );
            }

            #[no_mangle]
            /// load the input args and return it
            pub extern "C" fn [< $t:lower _input_args_echo_ret >](ptr: RemotePtr) -> RemotePtr {
                let r: test_common::[< $t:camel Type >] = host_args!(ptr);
                ret!(r);
            }
        }
    }
}

_s!(Bytes; vec![];);
_s!(String; "".to_string(););

macro_rules! _n {
    ( $t:tt; $n:ident; $inner:expr; $empty:expr; ) => {
        paste::item! {
            #[no_mangle]
            pub extern "C" fn [< $t:lower _serialize_n >](ptr: RemotePtr) -> RemotePtr {
                // build it
                let $n: test_common::IntegerType = host_args!(ptr);
                let s = paste::expr! {
                    test_common::[< $t:camel Type >]::from($inner)
                };
                // serialize it
                let _: SerializedBytes = s.try_into().unwrap();
                // return nothing
                ret!(
                    paste::expr! {
                        test_common::[< $t:camel Type >]::from($empty)
                    }
                );
            }

            #[no_mangle]
            pub extern "C" fn [< $t:lower _ret_n >](ptr: RemotePtr) -> RemotePtr {
                // build it
                let $n: test_common::IntegerType = host_args!(ptr);
                let s = paste::expr! {
                    test_common::[< $t:camel Type >]::from($inner)
                };
                // return it
                ret!(s);
            }
        }
    }
}

_n!(Bytes; n; vec![0; u32::from(n) as usize]; vec![];);
_n!(String; n; ".".repeat(u32::from(n) as usize).to_string(); "".to_string(););

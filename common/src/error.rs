#[derive(Debug)]
pub enum Error {
    // while converting pointers and lengths between u64 and i64 across the host/guest
    // we hit either a negative number (cannot fit in u64) or very large number (cannot fit in i64)
    // negative pointers and lengths are almost certainly indicative of a critical bug somewhere
    // max i64 represents about 9.2 exabytes so should keep us going long enough to patch wasmer
    // if commercial hardware ever threatens to overstep this limit
    PointerMap,
    // while shuffling raw bytes back and forward between Vec<u8> and utf-8 str values we have hit
    // an invalid utf-8 string.
    // in normal operations this is always a critical bug as the same rust internals are
    // responsible for bytes and utf-8 in both directions.
    // it is also possible that someone tries to throw invalid utf-8 at us to be evil.
    Utf8,
    // something went wrong while writing or reading bytes to/from wasm memory
    // this means something like "reading 16 bytes did not produce 2x u64 ints"
    // or maybe even "failed to write a byte to some pre-allocated wasm memory"
    // whatever this is it is very bad and probably not recoverable
    Memory,
}

impl From<std::num::TryFromIntError> for Error {
    fn from(_: std::num::TryFromIntError) -> Self {
        Error::PointerMap
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(_: std::str::Utf8Error) -> Self {
        Error::Utf8
    }
}

impl From<byte_slice_cast::Error> for Error {
    fn from(_: byte_slice_cast::Error) -> Self {
        Error::Memory
    }
}

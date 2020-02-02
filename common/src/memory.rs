// pub type StringPtr = *const u8;
// pub type Length = usize;
// pub type StringCap = usize;
// pub type PtrLen = u64;

/// Allocation is a 2 item u64 slice of offset/length
pub const ALLOCATION_ITEMS: usize = 2;
pub type Allocation = [u64; ALLOCATION_ITEMS];

/// Need Allocation to be u8 to copy as bytes across host/guest
pub const ALLOCATION_BYTES_ITEMS: usize = 16;
pub type AllocationBytes = [u8; ALLOCATION_BYTES_ITEMS];

/// Treat all pointers as u64 to sync across host/guest
pub type Ptr = u64;
/// Treat all lengths of memory as u64 to match Ptr
pub type Len = u64;
/// AllocationPtr is a pointer to an allocation
pub type AllocationPtr = u64;

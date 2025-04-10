pub struct RaydiumPool {
    pub pool_address: String,
    pub pool_type: RaydiumPoolType, // Enum for AMM or CLMM
    // Other pool data...
}

pub enum RaydiumPoolType {
    AMM,
    CLMM,
}

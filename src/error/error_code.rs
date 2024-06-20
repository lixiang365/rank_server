



// 用户错误
// 11xxx
pub const USER_NOT_FOUND: u32 = 11001;
pub const USER_ALREADY_EXISTS: u32 = 11002;
pub const INVALID_PASSWORD: u32 = 11003;

// token错误
// 12xxx
pub const INVALID_TOKEN: u32 = 12001;
pub const TOKEN_EXPIRED: u32 = 12002;
pub const MISSING_TOKEN: u32 = 12003;
pub const TOKEN_CREATION_ERROR: u32 = 12003;

// db错误
// 13xxx
pub const SOMETHING_WENT_WRONG: u32 = 13001;
pub const UNIQUE_CONSTRAINT_VIOLATION: u32 = 13002;

// request错误
// 20xxx
pub const VALIDATION_ERROR: u32 = 20001;
pub const JSON_REJECTION: u32 = 20002;
pub const SIGNATURE_ERROR: u32 = 20003;
pub const COMMON_REQUEST_ERROR: u32 = 20004;

// api错误
// 22xxx
pub const TOKEN_ERROR: u32 = 22001;
pub const USER_ERROR: u32 = 22002;
pub const DB_ERROR: u32 = 22003;
#[derive(Debug, Clone)]
pub struct DownloadCommand {
    pub file: String,
    pub begin: u64,
    pub end: u64,
}

#[derive(Debug, Clone)]
pub struct DeleteCommand {
    pub file: String,
}

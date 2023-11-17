use std::path::PathBuf;

use tokio::fs::DirBuilder;

use crate::{errors::RustusResult, models::file_info::FileInfo};

use super::base::InfoStorage;

#[derive(Clone)]
pub struct FileInfoStorage {
    info_dir: PathBuf,
}

impl FileInfoStorage {
    pub fn new(info_dir: PathBuf) -> Self {
        Self { info_dir }
    }

    pub fn info_file_path(&self, file_id: &str) -> PathBuf {
        self.info_dir.join(format!("{file_id}.info"))
    }
}

impl InfoStorage for FileInfoStorage {
    async fn prepare(&mut self) -> RustusResult<()> {
        if !self.info_dir.exists() {
            DirBuilder::new().create(self.info_dir.as_path()).await?;
        }
        Ok(())
    }

    async fn set_info(&self, file_info: &FileInfo, create: bool) -> RustusResult<()> {
        let info = file_info.clone();
        let path = self.info_file_path(info.id.as_str());
        let file = tokio::fs::OpenOptions::new()
            .write(true)
            .create(create)
            .truncate(true)
            .open(path)
            .await?;
        let str_data = serde_json::to_string(file_info)?;
        let mut writer = tokio::io::BufWriter::new(file);
        tokio::io::copy_buf(&mut str_data.as_bytes(), &mut writer).await?;
        Ok(())
    }

    async fn get_info(&self, file_id: &str) -> RustusResult<FileInfo> {
        let info_path = self.info_file_path(file_id);
        let file = tokio::fs::File::open(info_path).await?;
        let mut reader = tokio::io::BufReader::new(file);
        let mut contents: Vec<u8> = vec![];
        tokio::io::copy_buf(&mut reader, &mut contents).await?;
        Ok(serde_json::from_slice::<FileInfo>(contents.as_slice())?)
    }

    async fn remove_info(&self, file_id: &str) -> RustusResult<()> {
        let id = String::from(file_id);
        let info_path = self.info_file_path(id.as_str());
        tokio::fs::remove_file(info_path).await?;
        Ok(())
    }
}

use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use async_compression::tokio::bufread::GzipDecoder;
use futures_util::StreamExt;
use reqwest::Client;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Mutex;
use tokio_tar::Archive;
use tokio_util::io::StreamReader;
use tracing::info;

pub(crate) struct DeploymentInformation {
  pub(crate) full_name: String,
  pub(crate) token: String,
  pub(crate) commit_id: String,
  pub(crate) domain: String,
}

#[derive(Clone)]
pub(crate) struct DeploymentService {
  client: Client,
  webroot: PathBuf,
  root_domain: String,
  queue_receiver: Arc<Mutex<Receiver<DeploymentInformation>>>,
  queue_sender: Arc<Mutex<Sender<DeploymentInformation>>>,
}

impl DeploymentService {
  pub(crate) fn new(webroot: PathBuf, root_domain: String) -> Self {
    let (queue_sender, queue_receiver) = channel::<DeploymentInformation>(500);
    Self {
      client: Client::new(),
      webroot,
      root_domain,
      queue_receiver: Arc::new(Mutex::new(queue_receiver)),
      queue_sender: Arc::new(Mutex::new(queue_sender)),
    }
  }

  pub(crate) async fn queue_deployment(
    &mut self,
    data: DeploymentInformation,
  ) -> anyhow::Result<()> {
    Ok(self.queue_sender.lock().await.send(data).await?)
  }

  pub(crate) async fn deploy_loop(&self) -> anyhow::Result<()> {
    loop {
      let new_deployment = match self.queue_receiver.lock().await.recv().await {
        Some(value) => value,
        None => {
          continue;
        }
      };

      let dist = self
        .webroot
        .join(format!("{}.{}", new_deployment.domain, self.root_domain));

      if tokio::fs::try_exists(&dist).await? {
        info!("Cleaning existing {}", dist.to_str().unwrap_or("~invalid~"));
        tokio::fs::remove_dir_all(&dist).await?;
      }

      info!(
        "Deploying {}#{} into {}",
        new_deployment.full_name,
        new_deployment.commit_id,
        dist.to_str().unwrap_or("~invalid~")
      );

      let url = format!(
        "https://github.com/{}/tarball/{}",
        new_deployment.full_name, new_deployment.commit_id
      );

      let stream = self
        .client
        .get(url)
        .bearer_auth(new_deployment.token)
        .send()
        .await?
        .error_for_status()?
        .bytes_stream()
        .map(|x| Ok::<_, io::Error>(x.unwrap()));

      let reader = StreamReader::new(stream);
      let decoder = GzipDecoder::new(reader);
      let mut archive = Archive::new(decoder);

      archive.unpack(&dist).await?;

      // fixing stupid extra folder generated by github
      let mut dir = tokio::fs::read_dir(&dist).await?;
      let entry = dir.next_entry().await?.unwrap();
      let entry = entry.path();

      let mut child = tokio::fs::read_dir(&entry).await?;
      while let Some(path) = child.next_entry().await? {
        tokio::fs::rename(path.path(), dist.join(path.file_name())).await?;
      }

      tokio::fs::remove_dir(entry).await?;
    }
  }
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use crate::service::deploy::DeploymentService;

  #[tokio::test]
  async fn test_deployment_service() -> anyhow::Result<()> {
    let service = DeploymentService::new(PathBuf::from("."), "m4rc3l.de".to_string());

    service
      .deploy("MarcelCoding/zia", "abc", "main", "zia".to_string())
      .await
  }
}

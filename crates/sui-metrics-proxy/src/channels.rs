// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0
use anyhow::Result;
use axum::body::Bytes;
use tokio::sync::mpsc::Receiver;
use tracing::info;

/// NodeMetric is a placeholder for metric data
/// tbd in future PR
#[derive(Debug)]
pub struct NodeMetric {
    pub label: String, // ???
    pub data: Bytes,   // raw post data from node
}

/// An UpstreamConsumer accepts bytes from calling clients and is
/// responsible for sending them on to upstream services to store
/// the relayed metric
pub struct UpstreamConsumer {
    pub receiver: Receiver<NodeMetric>,
}

impl UpstreamConsumer {
    pub fn new(receiver: Receiver<NodeMetric>) -> Self {
        Self { receiver }
    }
    pub async fn run(&mut self) -> Result<()> {
        while let Some(metrics) = self.receiver.recv().await {
            info!("Receiver: {:?}", &metrics);
        }
        Ok(())
    }
}

// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0
use axum::{
    body::Body,
    extract::State,
    http::{header, Request, StatusCode},
    response::IntoResponse,
};
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tracing::error;

use crate::channels::NodeMetric;

/// Publish handler which receives metrics from nodes.  Nodes will call us at this endpoint
/// and we relay them to the upstream tsdb
///
/// An mpsc is used within this handler so that we can immediately return an accept to calling nodes.
/// Downstream processing failures may still result in metrics being dropped.
pub async fn publish_metrics(
    State(state): State<Arc<Sender<NodeMetric>>>,
    request: Request<Body>,
) -> impl IntoResponse {
    let bytes = match hyper::body::to_bytes(request.into_body()).await {
        Ok(bytes) => bytes,
        Err(_e) => {
            return (
                StatusCode::BAD_REQUEST,
                [(header::CONNECTION, "close")],
                "unable to extract post body",
            );
        }
    };
    let n = NodeMetric {
        label: "some_label".into(),
        data: bytes,
    };
    let sender = state.clone();
    if let Err(e) = sender.send(n).await {
        error!("unable to queue; unable to send to consumer; {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(header::CONNECTION, "close")],
            "unable to queue metrics",
        );
    }
    (StatusCode::OK, [(header::CONNECTION, "close")], "accepted")
}

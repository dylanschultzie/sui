// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0
use crate::channels::UpstreamConsumer;
use crate::handlers::publish_metrics;
use anyhow::Result;
use axum::{routing::post as axum_post, Router};
use fastcrypto::ed25519::Ed25519KeyPair;
use fastcrypto::traits::KeyPair;
use std::sync::Arc;
use std::time::Duration;
use sui_tls::{
    rustls::ServerConfig, SelfSignedCertificate, TlsAcceptor, ValidatorAllowlist,
    ValidatorCertVerifier,
};
use tokio::{signal, sync::mpsc};
use tracing::info;

/// Configure our graceful shutdown scenarios
pub async fn shutdown_signal(h: axum_server::Handle) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    let grace = 30;
    info!(
        "signal received, starting graceful shutdown, grace period {} seconds, if needed",
        &grace
    );
    h.graceful_shutdown(Some(Duration::from_secs(grace)))
}

/// App will configure our routes and create our mpsc channels.  This fn is also used to instrument
/// our tests
pub fn app(buffer_size: usize) -> Router {
    // we accept data on our UpstreamConsumer up to our buffer size.
    let (sender, receiver) = mpsc::channel(buffer_size);
    let mut consumer = UpstreamConsumer::new(receiver);

    tokio::spawn(async move { consumer.run().await });

    // build our application with a route and our sender mpsc
    Router::new()
        .route("/publish/metrics", axum_post(publish_metrics))
        .with_state(Arc::new(sender))
}

pub async fn server(
    listener: std::net::TcpListener,
    acceptor: TlsAcceptor,
    app: Router,
) -> std::io::Result<()> {
    // setup our graceful shutdown
    let handle = axum_server::Handle::new();
    // Spawn a task to gracefully shutdown server.
    tokio::spawn(shutdown_signal(handle.clone()));

    axum_server::Server::from_tcp(listener)
        .acceptor(acceptor)
        .handle(handle)
        .serve(app.into_make_service())
        .await
}

pub fn create_server_cert(
    hostname: &str,
) -> Result<(ServerConfig, ValidatorAllowlist), sui_tls::rustls::Error> {
    let mut rng = rand::thread_rng();
    let server_keypair = Ed25519KeyPair::generate(&mut rng);
    let server_certificate = SelfSignedCertificate::new(server_keypair.private(), hostname);

    ValidatorCertVerifier::rustls_server_config(
        vec![server_certificate.rustls_certificate()],
        server_certificate.rustls_private_key(),
    )
}

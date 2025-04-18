use lapin::{ChannelState, ConnectionProperties, ConnectionState};

#[derive(Clone)]
pub struct ConnnectionPool {
    url: String,
    properties: ConnectionProperties,
}

#[derive(Clone, Debug)]
pub struct ChannelPool {
    pool: mobc::Pool<ConnnectionPool>,
}

impl ChannelPool {
    #[must_use]
    pub const fn new(pool: mobc::Pool<ConnnectionPool>) -> Self {
        Self { pool }
    }
}

impl ConnnectionPool {
    #[must_use]
    pub const fn new(url: String, properties: ConnectionProperties) -> Self {
        Self { url, properties }
    }
}

#[async_trait::async_trait]
impl mobc::Manager for ConnnectionPool {
    type Connection = lapin::Connection;
    type Error = lapin::Error;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        let conn = lapin::Connection::connect(&self.url, self.properties.clone()).await?;
        Ok(conn)
    }

    async fn check(&self, conn: Self::Connection) -> Result<Self::Connection, Self::Error> {
        match conn.status().state() {
            ConnectionState::Connected => Ok(conn),
            other_state => Err(Self::Error::InvalidConnectionState(other_state)),
        }
    }
}

#[async_trait::async_trait]
impl mobc::Manager for ChannelPool {
    type Connection = lapin::Channel;
    type Error = lapin::Error;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        let conn = self.pool.get().await;
        match conn {
            Ok(inner) => inner.create_channel().await,
            Err(mobc::Error::Inner(inner)) => Err(inner),
            _ => Err(lapin::Error::ChannelsLimitReached),
        }
    }

    async fn check(&self, conn: Self::Connection) -> Result<Self::Connection, Self::Error> {
        match conn.status().state() {
            ChannelState::Connected => Ok(conn),
            other_state => Err(Self::Error::InvalidChannelState(other_state)),
        }
    }
}

use lapin::{ChannelState, ConnectionProperties, ConnectionState};

pub struct ConnnectionPool {
    url: String,
    properties: ConnectionProperties,
}

pub struct ChannelPool {
    pool: mobc::Pool<ConnnectionPool>,
}

impl ChannelPool {
    #[must_use]
    pub fn new(pool: mobc::Pool<ConnnectionPool>) -> Self {
        ChannelPool { pool }
    }
}

impl ConnnectionPool {
    #[must_use]
    pub fn new(url: String, properties: ConnectionProperties) -> Self {
        ConnnectionPool { url, properties }
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
        match self.pool.get().await {
            Ok(conn) => conn.create_channel().await,
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

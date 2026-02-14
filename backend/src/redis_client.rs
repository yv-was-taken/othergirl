use redis::Client;

pub fn connect(redis_url: &str) -> Result<Client, redis::RedisError> {
    Client::open(redis_url)
}

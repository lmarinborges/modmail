use std::num::ParseIntError;

use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::{
        channel::{ChannelType, PartialChannel},
        gateway::Ready,
        guild::Role,
        id::{ChannelId, GuildId, RoleId},
        interactions::application_command::{ApplicationCommandOptionType, ApplicationCommandType},
    },
};
use sqlx::{sqlite::SqliteQueryResult, FromRow, SqlitePool};

pub struct Bot {
    guild: u64,
    pool: SqlitePool,
}

impl Bot {
    pub fn new(pool: SqlitePool, guild: u64) -> Self {
        Self { pool, guild }
    }

    async fn config(&self, key: &str) -> sqlx::Result<Option<String>> {
        Ok(sqlx::query!("SELECT value FROM config WHERE key = ?", key)
            .fetch_optional(&self.pool)
            .await?
            .map(|r| r.value))
    }

    async fn set_config(&self, key: &str, value: &str) -> sqlx::Result<SqliteQueryResult> {
        sqlx::query!(
            "INSERT INTO config (key, value) VALUES (?, ?)
            ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            key,
            value
        )
        .execute(&self.pool)
        .await
    }

    async fn unset_config(&self, key: &str) -> sqlx::Result<SqliteQueryResult> {
        sqlx::query!("DELETE FROM config WHERE key = ?", key)
            .execute(&self.pool)
            .await
    }

    async fn get_blockrole(&self) -> sqlx::Result<Option<RoleId>> {
        let raw = self.config("blockrole").await?;
        Ok(raw.map(|s| RoleId(s.parse().expect("got malformed ID from database"))))
    }

    async fn set_blockrole(&self, role: &Role) -> sqlx::Result<SqliteQueryResult> {
        let id = role.id.0.to_string();
        self.set_config("blockrole", &id).await
    }

    async fn unset_blockrole(&self) -> sqlx::Result<SqliteQueryResult> {
        self.unset_config("blockrole").await
    }

    async fn get_inbox(&self) -> sqlx::Result<Option<ChannelId>> {
        let raw = self.config("inbox").await?;
        Ok(raw.map(|s| ChannelId(s.parse().expect("got malformed ID from database"))))
    }

    async fn set_inbox(&self, channel: &PartialChannel) -> sqlx::Result<SqliteQueryResult> {
        let id = channel.id.0.to_string();
        self.set_config("inbox", &id).await
    }

    async fn unset_inbox(&self) -> sqlx::Result<SqliteQueryResult> {
        self.unset_config("inbox").await
    }
}

#[async_trait]
impl EventHandler for Bot {
    async fn ready(&self, ctx: Context, _: Ready) {
        GuildId(self.guild)
            .set_application_commands(&ctx, |cmds| {
                cmds.create_application_command(|cmd| {
                    cmd.name("block")
                        .description("Block a user from using the bot.")
                        .kind(ApplicationCommandType::ChatInput)
                        .create_option(|opt| {
                            opt.name("codename")
                                .description("The codename. Must be an exact match.")
                                .kind(ApplicationCommandOptionType::String)
                                .required(true)
                        })
                })
                .create_application_command(|cmd| {
                    cmd.name("blockrole")
                        .description("Manage the role given to blocked users.")
                        .kind(ApplicationCommandType::ChatInput)
                        .create_option(|opt| {
                            opt.name("set")
                                .description("Set the role given to blocked users.")
                                .kind(ApplicationCommandOptionType::SubCommand)
                                .create_sub_option(|sub| {
                                    sub.name("role")
                                        .description("The role to be used.")
                                        .kind(ApplicationCommandOptionType::Role)
                                        .required(true)
                                })
                        })
                        .create_option(|opt| {
                            opt.name("unset")
                                .description("Unset the block role.")
                                .kind(ApplicationCommandOptionType::SubCommand)
                        })
                })
                .create_application_command(|cmd| {
                    cmd.name("inbox")
                        .description("Manage the channel threads will be added to.")
                        .kind(ApplicationCommandType::ChatInput)
                        .create_option(|opt| {
                            opt.name("set")
                                .description("Set the channel threads will be added to.")
                                .kind(ApplicationCommandOptionType::SubCommand)
                                .create_sub_option(|sub| {
                                    sub.name("channel")
                                        .description("The channel to be used. Must allow threads.")
                                        .kind(ApplicationCommandOptionType::Channel)
                                        .channel_types(&[ChannelType::Text])
                                        .required(true)
                                })
                        })
                        .create_option(|opt| {
                            opt.name("unset")
                                .description("Unset the inbox channel.")
                                .kind(ApplicationCommandOptionType::SubCommand)
                        })
                })
                .create_application_command(|cmd| {
                    cmd.name("close")
                        .description("Close this thread and forget the attached user.")
                        .kind(ApplicationCommandType::ChatInput)
                        .create_option(|opt| {
                            opt.name("codename")
                                .description("The codename. Must be an exact match.")
                                .kind(ApplicationCommandOptionType::String)
                                .required(true)
                        })
                })
            })
            .await
            .expect("failed to register commands");
    }
}

#[derive(FromRow)]
struct RawThread {
    codename: String,
    thread: String,
    user: String,
}

struct Thread {
    codename: String,
    thread: u64,
    user: u64,
}

impl TryFrom<RawThread> for Thread {
    type Error = ParseIntError;

    fn try_from(value: RawThread) -> Result<Self, Self::Error> {
        Ok(Self {
            codename: value.codename,
            thread: value.thread.parse()?,
            user: value.user.parse()?,
        })
    }
}

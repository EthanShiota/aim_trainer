#![allow(unused)]
use std::fs;

use bevy::{
    ecs::world::CommandQueue,
    prelude::*,
    tasks::{AsyncComputeTaskPool, IoTaskPool, Task, futures::check_ready},
};
use sqlx::{Pool, Sqlite, SqlitePool, prelude::*, sqlite::SqlitePoolOptions};
pub mod commands;

pub struct DatabasePlugin;

#[derive(Resource)]
pub struct DatabasePool(pub SqlitePool);

#[derive(Component)]
struct DatabaseTask(pub Task<Pool<Sqlite>>);

#[derive(Component)]
struct AsyncCommandQueue(Task<CommandQueue>);

impl Plugin for DatabasePlugin {
    fn build(&self, app: &mut App) {
        // INFO: Assume sqlite db is in cwd
        if !std::fs::exists("./data.db").unwrap() {
            std::fs::File::create("./data.db").unwrap();
        }
        app.add_systems(Startup, setup).add_systems(Update, poll);
    }
}

fn poll(mut tasks: Query<(Entity, &mut AsyncCommandQueue)>, mut commands: Commands) {
    for (ent, mut t) in &mut tasks {
        if let Some(mut pool) = check_ready(&mut t.0) {
            commands.append(&mut pool);
            commands.entity(ent).despawn();
        }
    }
}

fn setup(mut commands: Commands) -> Result {
    let compute = IoTaskPool::get();
    let task = compute.spawn(async {
        let mut command_queue = CommandQueue::default();
        let pool = SqlitePool::connect("sqlite:data.db").await.unwrap();
        run_migrations(&pool).await;
        command_queue.push(move |world: &mut World| {
            world.insert_resource(DatabasePool(pool));
        });
        command_queue
    });
    commands.spawn(AsyncCommandQueue(task));
    Ok(())
}

async fn run_migrations(pool: &Pool<Sqlite>) {
    let migrator = sqlx::migrate!();
    migrator.run(pool).await.unwrap();
}

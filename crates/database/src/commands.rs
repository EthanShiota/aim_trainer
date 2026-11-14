use bevy::{prelude::*, tasks::IoTaskPool};

pub struct InsertScore {
    pub accuracy: f32,
    pub score: u32,
    pub beatmap_id: u32,
}

impl Command for InsertScore {
    type Out = ();
    fn apply(self, world: &mut World) -> Self::Out {
        let pool = world
            .get_resource::<crate::DatabasePool>()
            .unwrap()
            .0
            .clone();
        let task_pool = IoTaskPool::get();
        task_pool
            .spawn(async move {
                sqlx::query(
                    r#"
            INSERT INTO scores (beatmap_id, accuracy, score) VALUES (?1, ?2, ?3)
            "#,
                )
                .bind(self.beatmap_id)
                .bind(self.accuracy)
                .bind(self.score)
                .execute(&pool)
                .await
                .unwrap();
            })
            .detach();
    }
}

pub struct InsertBeatmap {
    pub beatmap_id: u32,
    pub title: String,
    pub version: String,
}

impl Command for InsertBeatmap {
    type Out = ();
    fn apply(self, world: &mut World) -> Self::Out {
        let pool = world
            .get_resource::<crate::DatabasePool>()
            .unwrap()
            .0
            .clone();
        let task_pool = IoTaskPool::get();
        task_pool
            .spawn(async move {
                sqlx::query(
                    r#"
            INSERT OR IGNORE INTO beatmaps (beatmap_id, title, version) VALUES (?1, ?2, ?3)
            "#,
                )
                .bind(self.beatmap_id)
                .bind(self.title)
                .bind(self.version)
                .execute(&pool)
                .await
                .unwrap();
            })
            .detach();
    }
}

use actix_web::web;
use super::monster_apis::{get_monsters, get_monster_by_id, create_monster, update_monster_by_id, delete_monster_by_id, import_csv};
use super::battle_apis::{get_battles, get_battle_by_id, delete_battle_by_id, create_battle};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .service(get_monsters)
            .service(create_monster)
            .service(get_monster_by_id)
            .service(delete_monster_by_id)
            .service(update_monster_by_id)
            .service(import_csv)
            .service(get_battles)
            .service(get_battle_by_id)
            .service(delete_battle_by_id)
            .service(create_battle)
    );
}
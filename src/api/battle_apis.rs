use actix_web::{web, get, post, delete, HttpResponse};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use crate::{models::battle::Battle, repository::database::Database};
use crate::models::monster::{self, Monster};
use crate::repository::battle_repository;
use crate::repository::monster_repository;

#[derive(Serialize, Deserialize)]
pub struct CreateBattleRequest {
    monster_a: Option<String>,
    monster_b: Option<String>,
}

#[post("/battles")]
pub async fn create_battle(db: web::Data<Database>, battle_request: web::Json<CreateBattleRequest>) -> HttpResponse {
    let monster_a_id = match &battle_request.monster_a {
        Some(id) => id,
        None => return HttpResponse::BadRequest().json("Monster A id is required")
    };
    let monster_b_id = match &battle_request.monster_b {
        Some(id) => id,
        None => return HttpResponse::BadRequest().json("Monster B id is required")
    };
    
    let monster_a = match monster_repository::get_monster_by_id(&db, &monster_a_id) {
        Some(monster) => monster,
        None => return HttpResponse::BadRequest().json("Monster A id not found") 
    };
    let monster_b = match monster_repository::get_monster_by_id(&db, &monster_b_id) {
        Some(monster) => monster,
        None => return HttpResponse::BadRequest().json("Monster B id not found") 
    };

    let winner =  simulate_battle(monster_a, monster_b).id;
    let battle = Battle {
        id: uuid::Uuid::new_v4().to_string(),
        monster_a: monster_a_id.clone(),
        monster_b: monster_b_id.clone(),
        winner: winner,
        created_at: None,
        updated_at: None
    };

    match battle_repository::create_battle(&db, battle) {
        Ok(battle) => HttpResponse::Created().json(battle),
        Err(e) => HttpResponse::InternalServerError().json(e.to_string())
    }
}

#[get("/battles")]
pub async fn get_battles(db: web::Data<Database>) -> HttpResponse {
    let battles = battle_repository::get_battles(&db);
    HttpResponse::Ok().json(battles)
}

#[get("/battles/{id}")]
pub async fn get_battle_by_id(db: web::Data<Database>, id: web::Path<String>) -> HttpResponse {
    let battle = battle_repository::get_battle_by_id(&db, &id);
    match battle {
        Some(battle) => HttpResponse::Ok().json(battle),
        None => HttpResponse::NotFound().json("Battle not found"),
    }
}

#[delete("/battles/{id}")]
pub async fn delete_battle_by_id(db: web::Data<Database>, id: web::Path<String>) -> HttpResponse {
    match battle_repository::delete_battle_by_id(&db, &id) {
        Some(_) => HttpResponse::NoContent().finish(),
        None => HttpResponse::NotFound().json("Battle not found"),
    }
}

/*
- The monster with the highest speed makes the first attack, if both speeds are equal, the monster with the higher attack goes first.
- For calculating the damage, subtract the defense from the attack (attack - defense); the difference is the damage; 
- if the attack is equal to or lower than the defense, the damage is 1.
Subtract the damage from the HP (HP = HP - damage).
Monsters will battle in turns until one wins; all turns should be calculated in the same request; for that reason, the battle endpoint should return winner data in just one call.
Who wins the battle is the monster who subtracted the enemy’s HP to zero
*/
fn simulate_battle(mut monster_a: Monster, mut monster_b: Monster) -> Monster {

    let mut monster_a_turn = if monster_a.speed > monster_b.speed || 
                                (monster_a.speed == monster_b.speed && monster_a.attack > monster_b.attack) {
        true
    } else {
        false
    };

    loop {
        let (attacker, defender) = if monster_a_turn {
            (&mut monster_a, &mut monster_b)
        } else {
            (&mut monster_b, &mut monster_a)
        };

        let damage = if attacker.attack > defender.defense {
            attacker.attack - defender.defense
        } else {
            1
        };

        if defender.hp > damage {
            defender.hp -= damage;
        } else {
            defender.hp = 0;
            return attacker.clone()
        }

        monster_a_turn = !monster_a_turn;
    }
}

#[cfg(test)]
mod tests {
    use actix_web::{test, http, App};
    use actix_web::web::Data;
    use crate::{
        utils::test_utils::init_test_battle,
        utils::test_utils::init_test_monsters
    };
    use serde_json;

    use super::*;

    #[actix_rt::test]
    async fn test_should_get_all_battles_correctly() {
        let db = Database::new();
        let app = App::new().app_data(Data::new(db)).service(get_battles);

        let mut app = test::init_service(app).await;

        let req = test::TestRequest::get().uri("/battles").to_request();
        let resp = test::call_service(&mut app, req).await;
        
        assert!(resp.status().is_success());
    }

    #[actix_rt::test]
    async fn test_should_get_404_error_if_battle_does_not_exists() {
        let db = Database::new();
        let app = App::new().app_data(Data::new(db)).service(get_battle_by_id);

        let mut app = test::init_service(app).await;

        let req = test::TestRequest::get().uri("/battles/123").to_request();
        let resp = test::call_service(&mut app, req).await;
        
        assert_eq!(resp.status(), http::StatusCode::NOT_FOUND)
    }

    #[actix_rt::test]
    async fn test_should_get_a_single_battle_correctly() {
        let mut db = Database::new();
        let test_battle = init_test_battle(&mut db).await;
        let app = App::new().app_data(Data::new(db)).service(get_battle_by_id);

        let mut app = test::init_service(app).await;

        let req = test::TestRequest::get().uri(&format!("/battles/{}", test_battle.id)).to_request();
        let resp = test::call_service(&mut app, req).await;
        
        assert!(resp.status().is_success());
    }

    #[actix_rt::test]
    async fn test_should_delete_a_battle_correctly() {
        let mut db = Database::new();
        let test_battle = init_test_battle(&mut db).await;
        let app = App::new().app_data(Data::new(db)).service(delete_battle_by_id);

        let mut app = test::init_service(app).await;

        let req = test::TestRequest::delete().uri(&format!("/battles/{}", test_battle.id)).to_request();
        let resp = test::call_service(&mut app, req).await;
        
        assert_eq!(resp.status(), http::StatusCode::NO_CONTENT);
    }

    #[actix_rt::test]
    async fn test_should_create_a_battle_with_404_error_if_one_parameter_has_a_monster_id_does_not_exists() {
        let mut db = Database::new();
        let test_monsters = init_test_monsters(&mut db).await;

        let app = App::new().app_data(Data::new(db)).service(create_battle);

        let mut app = test::init_service(app).await;

        let battle_request = CreateBattleRequest {
            monster_a: Some("123".to_string()),
            monster_b: Some(test_monsters[0].id.clone()),
        };
        let req = test::TestRequest::post()
            .uri("/battles")
            .set_json(&battle_request)
            .to_request();
        let resp = test::call_service(&mut app, req).await;
        
        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    }

    #[actix_rt::test]
    async fn test_should_create_a_battle_with_a_bad_request_response_if_one_parameter_is_null() {
        let mut db = Database::new();
        let test_monsters = init_test_monsters(&mut db).await;

        let app = App::new().app_data(Data::new(db)).service(create_battle);

        let mut app = test::init_service(app).await;

        let battle_request = CreateBattleRequest {
            monster_a: None,
            monster_b: Some(test_monsters[0].id.clone()),
        };
        let req = test::TestRequest::post()
            .uri("/battles")
            .set_json(&battle_request)
            .to_request();
        let resp = test::call_service(&mut app, req).await;
        
        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    }

    #[actix_rt::test]
    async fn test_should_create_battle_correctly_with_monster_a_winning() {
        let mut db = Database::new();
        let test_monsters = init_test_monsters(&mut db).await;

        let app = App::new().app_data(Data::new(db)).service(create_battle);

        let mut app = test::init_service(app).await;

        let battle_request = CreateBattleRequest {
            monster_a: Some(test_monsters[6].id.clone()),
            monster_b: Some(test_monsters[5].id.clone()),
        };
        let req = test::TestRequest::post()
            .uri("/battles")
            .set_json(&battle_request)
            .to_request();
        let resp = test::call_service(&mut app, req).await;
        let battle: Battle = test::read_body_json(resp).await;

        assert_eq!(battle.monster_a, battle.winner);
    }

    #[actix_rt::test]
    async fn test_should_create_battle_correctly_with_monster_b_winning_if_theirs_speeds_same_and_monster_b_has_higher_attack() {
        let mut db = Database::new();
        let test_monsters = init_test_monsters(&mut db).await;

        let app = App::new().app_data(Data::new(db)).service(create_battle);

        let mut app = test::init_service(app).await;

        let battle_request = CreateBattleRequest {
            monster_a: Some(test_monsters[4].id.clone()),
            monster_b: Some(test_monsters[3].id.clone()),
        };
        let req = test::TestRequest::post()
            .uri("/battles")
            .set_json(&battle_request)
            .to_request();
        let resp = test::call_service(&mut app, req).await;
        let battle: Battle = test::read_body_json(resp).await;

        assert_eq!(battle.monster_b, battle.winner);
    }

}


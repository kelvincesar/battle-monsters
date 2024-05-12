use actix_web::{web, get, post, delete, put, HttpResponse, Error};
use actix_multipart::Multipart;
use futures::TryStreamExt;
use tempfile::NamedTempFile;
use std::io::Write;
use crate::{models::monster::Monster, repository::database::Database};
use crate::repository::monster_repository;

#[get("/monsters")]
pub async fn get_monsters(db: web::Data<Database>) -> HttpResponse {
    let monsters = monster_repository::get_monsters(&db);
    HttpResponse::Ok().json(monsters)
}

#[post("/monsters")]
pub async fn create_monster(db: web::Data<Database>, new_monster: web::Json<Monster>) -> HttpResponse {
    let monster = monster_repository::create_monster(&db, new_monster.into_inner());
    match monster {
        Ok(monster) => HttpResponse::Created().json(monster),
        Err(err) => HttpResponse::InternalServerError().json(err.to_string()),
    }
}

#[get("/monsters/{id}")]
pub async fn get_monster_by_id(db: web::Data<Database>, id: web::Path<String>) -> HttpResponse {
    let monster = monster_repository::get_monster_by_id(&db, &id);
    match monster {
        Some(monster) => HttpResponse::Ok().json(monster),
        None => HttpResponse::NotFound().json("Monster not found"),
    }
}

#[delete("/monsters/{id}")]
pub async fn delete_monster_by_id(db: web::Data<Database>, id: web::Path<String>) -> HttpResponse {
    let monster = monster_repository::delete_monster_by_id(&db, &id);
    match monster {
        Some(_) => HttpResponse::NoContent().finish(),
        None => HttpResponse::NotFound().json("Monster not found"),
    }
}

#[put("/monsters/{id}")]
pub async fn update_monster_by_id(db: web::Data<Database>, id: web::Path<String>, updated_monster: web::Json<Monster>) -> HttpResponse {
    let monster = monster_repository::update_monster_by_id(&db, &id, updated_monster.into_inner());
    match monster {
        Some(monster) => HttpResponse::Ok().json(monster),
        None => HttpResponse::NotFound().json("Monster not found"),
    }
}

#[post("/monsters/import_csv")]
pub async fn import_csv(db: web::Data<Database>, mut payload: Multipart) -> Result<HttpResponse, Error> {
    let mut file_name: Option<String> = None;
    let mut temp_file: Option<NamedTempFile> = None;
    let mut new_monsters: Vec<Monster> = Vec::new();

    while let Some(mut field) = payload.try_next().await? {
        let content_disposition = field.content_disposition();

        if let Some(name) = content_disposition.get_filename() {
            file_name = Some(name.to_string());
            temp_file = Some(NamedTempFile::new().unwrap());

            while let Some(chunk) = field.try_next().await? {
                temp_file.as_mut().unwrap().write_all(&chunk).unwrap();
            }
        } else {
            return Ok(HttpResponse::BadRequest().json("No file name provided"));
        }
    }

    if let Some(_file_name) = file_name {
        if let Some(temp_file) = temp_file {
            let mut reader = csv::ReaderBuilder::new()
                .has_headers(true)
                .from_path(temp_file.path())
                .unwrap();

                
                for result in reader.deserialize::<Monster>() {
                    match result {
                        Ok(monster) => {
                            new_monsters.push(monster);
                        }
                        Err(e) => {
                            println!("Reader: {:?}", e.to_string());
                            return Ok(HttpResponse::BadRequest().json("Incomplete data, check your file."));
                        }
                    }
                }
    
                if new_monsters.is_empty() {
                    return Ok(HttpResponse::BadRequest().json("No valid monsters found in the CSV file"));
                }

            let results: Vec<Result<Monster, String>> = new_monsters
            .iter()
            .map(|new_monster| {
                match monster_repository::create_monster(&db, new_monster.clone()) {
                    Ok(monster) => Ok(monster),
                    Err(err) => Err(err.to_string()),
                }
            })
            .collect();
    

            let (successes, _errors): (Vec<_>, Vec<_>) = results.into_iter().partition(Result::is_ok);

            let successful_monsters: Vec<Monster> = successes.into_iter().map(Result::unwrap).collect();

            if successful_monsters.is_empty() {
                return Ok(HttpResponse::InternalServerError().json("Failed to create monsters"));
            } else {
                return Ok(HttpResponse::Ok().json(successful_monsters));
            }
        }
    }

    Ok(HttpResponse::BadRequest().json("No file uploaded"))
}

#[cfg(test)]
mod tests {
    use actix_web::{test, http, App};
    use actix_web::web::Data;
    use crate::{
        utils::test_utils::init_test_monsters
    };

    use actix_multipart_test::MultiPartFormDataBuilder;
    use super::*;

    #[actix_rt::test]
    async fn test_should_get_all_monsters_correctly() {
        let db = Database::new();
        let app = App::new().app_data(Data::new(db)).service(get_monsters);

        let mut app = test::init_service(app).await;

        let req = test::TestRequest::get().uri("/monsters").to_request();
        let resp = test::call_service(&mut app, req).await;
        
        assert!(resp.status().is_success());
    }

    #[actix_rt::test]
    async fn test_should_get_404_error_if_monster_does_not_exists() {
        
        let db = Database::new();
        let app = App::new().app_data(Data::new(db)).service(get_monster_by_id);

        let mut app = test::init_service(app).await;

        let req = test::TestRequest::get().uri("/monsters/999999").to_request();

        let resp = test::call_service(&mut app, req).await;
        
        assert_eq!(resp.status(), http::StatusCode::NOT_FOUND);
    }

    #[actix_rt::test]
    async fn test_should_get_a_single_monster_correctly() {
        
        let mut db = Database::new();
        let test_monsters = init_test_monsters(&mut db).await;

        let app = App::new().app_data(Data::new(db)).service(get_monster_by_id);

        let mut app = test::init_service(app).await;

        let req = test::TestRequest::get()
        .uri(format!("/monsters/{}", test_monsters[0].id).as_str()).to_request();
        
        let resp = test::call_service(&mut app, req).await;
 
        assert_eq!(resp.status(), http::StatusCode::OK);
    }

    #[actix_rt::test]
    async fn test_should_create_a_new_monster() {
        let mut db = Database::new();
        let _test_monsters = init_test_monsters(&mut db).await;

        let app = App::new().app_data(Data::new(db)).service(create_monster);

        let mut app = test::init_service(app).await;

        let new_monster_data = Monster {
            id: _test_monsters[0].id.clone(),
            name: _test_monsters[0].name.clone(),
            image_url: _test_monsters[0].image_url.clone(),
            attack: _test_monsters[0].attack.clone(),
            defense: _test_monsters[0].defense.clone(),
            speed: _test_monsters[0].speed.clone(),
            hp: _test_monsters[0].hp.clone(),
            created_at: _test_monsters[0].created_at.clone(),
            updated_at: _test_monsters[0].updated_at.clone(),
        };

        let req = test::TestRequest::post()
        .uri("/monsters")
        .set_json(&new_monster_data)
        .to_request();

        let resp = test::call_service(&mut app, req).await;
 
        assert_eq!(resp.status(), http::StatusCode::CREATED);
    }

    #[actix_rt::test]
    async fn test_should_update_a_monster_correctly() {
        let mut db = Database::new();
        let _test_monsters = init_test_monsters(&mut db).await;

        let app = App::new().app_data(Data::new(db)).service(update_monster_by_id);

        let mut app = test::init_service(app).await;

        let update_monster_data = Monster {
            id: _test_monsters[0].id.clone(),
            name: "Update name of monster".to_string(),
            image_url: _test_monsters[0].image_url.clone(),
            attack: _test_monsters[0].attack.clone(),
            defense: _test_monsters[0].defense.clone(),
            speed: _test_monsters[0].speed.clone(),
            hp: _test_monsters[0].hp.clone(),
            created_at: _test_monsters[0].created_at.clone(),
            updated_at: _test_monsters[0].updated_at.clone(),
        };
        let req = test::TestRequest::put()
        .uri(format!("/monsters/{}", _test_monsters[0].id).as_str())
        .set_json(&update_monster_data)
        .to_request();
        
        let resp = test::call_service(&mut app, req).await;
 
        assert_eq!(resp.status(), http::StatusCode::OK);
    }

    #[actix_rt::test]
    async fn test_should_update_with_404_error_if_monster_does_not_exists() {
        let mut db = Database::new();
        let _test_monsters = init_test_monsters(&mut db).await;

        let app = App::new().app_data(Data::new(db)).service(update_monster_by_id);

        let mut app = test::init_service(app).await;

        let update_monster_data = Monster {
            id: _test_monsters[0].id.clone(),
            name: "Update name of monster".to_string(),
            image_url: _test_monsters[0].image_url.clone(),
            attack: _test_monsters[0].attack.clone(),
            defense: _test_monsters[0].defense.clone(),
            speed: _test_monsters[0].speed.clone(),
            hp: _test_monsters[0].hp.clone(),
            created_at: _test_monsters[0].created_at.clone(),
            updated_at: _test_monsters[0].updated_at.clone(),
        };
        let req = test::TestRequest::put()
        .uri(format!("/monsters/{}", 99999).as_str())
        .set_json(&update_monster_data)
        .to_request();
        
        let resp = test::call_service(&mut app, req).await;
 
        assert_eq!(resp.status(), http::StatusCode::NOT_FOUND);
    }

    #[actix_rt::test]
    async fn test_should_delete_a_monster_correctly() {
        
        let mut db = Database::new();
        let _test_monsters = init_test_monsters(&mut db).await;

        let app = App::new().app_data(Data::new(db)).service(delete_monster_by_id);

        let mut app = test::init_service(app).await;

        let req = test::TestRequest::delete()
        .uri(format!("/monsters/{}", _test_monsters[0].id).as_str()).to_request();
        
        let resp = test::call_service(&mut app, req).await;
 
        assert_eq!(resp.status(), http::StatusCode::NO_CONTENT);
    }

    #[actix_rt::test]
    async fn test_should_delete_with_404_error_if_monster_does_not_exists() {
        
        let mut db = Database::new();
        let _test_monsters = init_test_monsters(&mut db).await;

        let app = App::new().app_data(Data::new(db)).service(delete_monster_by_id);

        let mut app = test::init_service(app).await;

        let req = test::TestRequest::delete()
        .uri(format!("/monsters/{}", 99999).as_str()).to_request();
        
        let resp = test::call_service(&mut app, req).await;
 
        assert_eq!(resp.status(), http::StatusCode::NOT_FOUND);
    }

    #[actix_rt::test]
    async fn test_should_import_all_the_csv_objects_into_the_database_successfully() {
        let db = Database::new();
        let app = App::new().app_data(Data::new(db)).service(import_csv);

        let mut app = test::init_service(app).await;

        let mut multipart_form_data_builder = MultiPartFormDataBuilder::new();
        multipart_form_data_builder.with_file("./src/utils/files/monsters-correct.csv", "file", "text/csv", "monsters-correct.csv");

        let (header, body) = multipart_form_data_builder.build();

        let req = test::TestRequest::post()
            .uri("/monsters/import_csv")
            .insert_header(header)
            .set_payload(body)
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        let code = resp.status();
        assert_eq!(code, http::StatusCode::OK);
    }

    #[actix_rt::test]
    async fn test_should_fail_when_importing_csv_file_with_inexistent_columns() {
        let db = Database::new();
        let app = App::new().app_data(Data::new(db)).service(import_csv);

        let mut app = test::init_service(app).await;

        let mut multipart_form_data_builder = MultiPartFormDataBuilder::new();
        multipart_form_data_builder.with_file("./src/utils/files/monsters-wrong-column.csv", "file", "text/csv", "monsters-wrong-column.csv");

        let (header, body) = multipart_form_data_builder.build();

        let req = test::TestRequest::post()
            .uri("/monsters/import_csv")
            .insert_header(header)
            .set_payload(body)
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        let code = resp.status();
        assert_eq!(code, http::StatusCode::BAD_REQUEST);
    }
}
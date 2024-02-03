use actix_web::{
    get,
    web::{self, ServiceConfig},
    HttpRequest, HttpResponse, Responder,
};
use shuttle_actix_web::ShuttleActixWeb;
use tera::Tera;

struct AppState {
    tera: Tera,
}

#[get("/")]
async fn index() -> impl Responder {
    web::Redirect::to("/inventario").permanent()
}

#[get("/inventario")]
async fn inventario(req: HttpRequest, data: web::Data<AppState>) -> impl Responder {
    let mut ctx = tera::Context::new();
    ctx.insert("title", "Inventario");
    let rendered = data.tera.render("index.html", &ctx).unwrap();
    HttpResponse::Ok().body(rendered)
}

#[shuttle_runtime::main]
async fn main() -> ShuttleActixWeb<impl FnOnce(&mut ServiceConfig) + Send + Clone + 'static> {
    let tera = Tera::new("./static/**/*.html").unwrap();
    let app_state = web::Data::new(AppState { tera: tera.clone() });

    let config = move |cfg: &mut web::ServiceConfig| {
        cfg.app_data(app_state.clone())
            .service(index)
            .service(inventario);
    };

    Ok(config.into())
}

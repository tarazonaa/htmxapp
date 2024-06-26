use actix_files::NamedFile;
use actix_web::{delete, get, post, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use serde_derive::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tera::{Context, Tera};

#[derive(Serialize, Clone, Debug, Default)]
struct Contact {
    id: i32,
    first: String,
    last: String,
    phone: String,
    email: String,
    error: Option<String>,
}

struct AppState {
    contacts_vec: Arc<Mutex<Vec<Contact>>>,
    tera: Tera,
}

#[get("/")]
async fn index() -> impl Responder {
    let _ = NamedFile::open("static/js/htmx.js");
    web::Redirect::to("/contacts").permanent()
}

// LIST CONTACTS

#[get("/contacts")]
async fn contacts(req: HttpRequest, data: web::Data<AppState>) -> HttpResponse {
    let query_string = req.query_string();
    let query_params: HashMap<String, String> = web::Query::from_query(query_string)
        .unwrap_or_else(|_| web::Query(HashMap::new()))
        .into_inner();
    let q = query_params.get("q").cloned().unwrap_or_default();
    let page: usize = query_params
        .get("page")
        .cloned()
        .unwrap_or("1".to_string())
        .parse()
        .unwrap_or(1);
    let per_page: usize = 10; // Number of contacts per page
    let contacts = data.contacts_vec.lock().unwrap();
    let contacts_to_show = contacts
        .iter()
        .filter(|contact| {
            contact.first.to_lowercase().contains(&q.to_lowercase())
                || contact.last.to_lowercase().contains(&q.to_lowercase())
                || contact.phone.to_lowercase().contains(&q.to_lowercase())
                || contact.email.to_lowercase().contains(&q.to_lowercase())
        })
        .collect::<Vec<_>>();

    let total_pages = (contacts_to_show.len() + per_page - 1) / per_page; // Calculate total pages needed
    let start_index = (page - 1) * per_page; // Calculate start index
    let end_index = std::cmp::min(start_index + per_page, contacts_to_show.len()); // Calculate end index

    let page_contacts = &contacts_to_show[start_index..end_index]; // Get the slice of contacts for the current page

    let mut context = Context::new();
    context.insert("title", "Contacts");
    context.insert("q", &q);
    context.insert("contacts", &page_contacts);
    context.insert("page", &page);
    context.insert("total_pages", &total_pages);

    let mut body = data
        .tera
        .render("index.html", &context)
        .unwrap_or_else(|e| e.to_string());

    if req.headers().get("HX-Trigger-Name").is_some() {
        body = data.tera.render("rows.html", &context).unwrap();
    }

    HttpResponse::Ok().body(body)
}

#[get("/contacts/count")]
async fn contacts_count(data: web::Data<AppState>) -> impl Responder {
    let contacts_db = data.contacts_vec.lock().unwrap();
    let count = contacts_db.len();
    HttpResponse::Ok().body(format!("{}", count))
}

#[get("/contacts/{id}")]
async fn show_contact(data: web::Data<AppState>, path: web::Path<i32>) -> HttpResponse {
    let id = path.into_inner();
    let db = data.contacts_vec.lock().unwrap();
    let contact = db
        .iter()
        .find(|contact| contact.id == id)
        .unwrap_or_else(|| panic!("Contact with id {} not found in contacts: {:?}", id, db));
    let mut context = Context::new();
    context.insert("title", "Contact");
    context.insert("contact", &contact);
    let body = data.tera.render("show.html", &context).unwrap();
    HttpResponse::Ok().body(body)
}

// NEW CONTACT

#[get("/contacts/new")]
async fn new_contact(data: web::Data<AppState>) -> HttpResponse {
    let mut context = Context::new();
    let contact = Contact {
        id: 0,
        first: "".to_string(),
        last: "".to_string(),
        phone: "".to_string(),
        email: "".to_string(),
        error: None,
    };
    context.insert("title", "New Contact");
    context.insert("contact", &contact);
    let body = data.tera.render("new.html", &context).unwrap();
    HttpResponse::Ok().body(body)
}

#[post("/contacts/new")]
async fn create_contact(
    data: web::Data<AppState>,
    params: web::Form<HashMap<String, String>>,
) -> impl Responder {
    let mut contacts_db = data.contacts_vec.lock().unwrap_or_else(|e| e.into_inner());
    let id = contacts_db.len() as i32 + 1;
    let contact = Contact {
        id,
        first: params
            .get("first")
            .map(|s| s.to_string())
            .unwrap_or_else(|| "DefaultFirstName".to_string()),
        last: params
            .get("last")
            .map(|s| s.to_string())
            .unwrap_or_else(|| "DefaultLastName".to_string()),
        phone: params
            .get("phone")
            .map(|s| s.to_string())
            .unwrap_or_else(|| "DefaultPhone".to_string()),
        email: params
            .get("email")
            .map(|s| s.to_string())
            .unwrap_or_else(|| "DefaultEmail".to_string()),
        error: None,
    };
    contacts_db.push(contact);
    web::Redirect::to("/contacts").see_other()
}

// EDIT CONTACT

#[get("/contacts/{id}/edit")]
async fn edit_contact(data: web::Data<AppState>, path: web::Path<i32>) -> HttpResponse {
    let id = path.into_inner();
    let db = data.contacts_vec.lock().unwrap();
    let contact = db
        .iter()
        .find(|contact| contact.id == id)
        .unwrap_or_else(|| panic!("Contact with id {} not found in contacts: {:?}", id, db));
    let mut context = Context::new();
    context.insert("title", "Edit Contact");
    context.insert("contact", &contact);
    let body = data.tera.render("edit.html", &context).unwrap();
    HttpResponse::Ok().body(body)
}

#[post("/contacts/{id}/edit")]
async fn update_contact(
    data: web::Data<AppState>,
    path: web::Path<i32>,
    params: web::Form<HashMap<String, String>>,
) -> impl Responder {
    let id = path.into_inner();
    let mut contacts_db = data.contacts_vec.lock().unwrap_or_else(|e| e.into_inner());

    // First, find the position (index) of the contact
    let contact_pos = contacts_db
        .iter()
        .position(|contact| contact.id == id)
        .unwrap_or_else(|| panic!("Contact with id {} not found in contacts", id));

    // Then, access and update the contact at that position
    if let Some(contact) = contacts_db.get_mut(contact_pos) {
        contact.first = params
            .get("first")
            .map(|s| s.to_string())
            .unwrap_or_else(|| "DefaultFirstName".to_string());
        contact.last = params
            .get("last")
            .map(|s| s.to_string())
            .unwrap_or_else(|| "DefaultLastName".to_string());
        contact.phone = params
            .get("phone")
            .map(|s| s.to_string())
            .unwrap_or_else(|| "DefaultPhone".to_string());
        contact.email = params
            .get("email")
            .map(|s| s.to_string())
            .unwrap_or_else(|| "DefaultEmail".to_string());
    } else {
        panic!("Contact with id {} not found in contacts", id);
    }

    web::Redirect::to("/contacts").see_other()
}

// DELETE CONTACT
#[delete("/contacts/{id}")]
async fn delete_contact(data: web::Data<AppState>, path: web::Path<i32>) -> impl Responder {
    let id = path.into_inner();
    let mut contacts_db = data.contacts_vec.lock().unwrap_or_else(|e| e.into_inner());

    // First, find the position (index) of the contact
    if let Some(contact_pos) = contacts_db.iter().position(|contact| contact.id == id) {
        contacts_db.remove(contact_pos);
    } else {
        panic!("Contact with id {} not found in contacts", id);
    }

    web::Redirect::to("/contacts").see_other()
}

#[get("/contacts/{id}/email")]
async fn contacts_email_get(
    request: HttpRequest,
    data: web::Data<AppState>,
    path: web::Path<i32>,
) -> Result<String, Box<dyn std::error::Error>> {
    let id = path.into_inner();
    let query_params = web::Query::<HashMap<String, String>>::from_query(request.query_string())
        .unwrap_or_else(|_| web::Query(HashMap::new()))
        .into_inner();
    let email = query_params.get("email").unwrap();
    let db = data.contacts_vec.lock().unwrap();
    let contact = db
        .iter()
        .find(|contact| contact.id == id)
        .unwrap_or_else(|| panic!("Contact with id {} not found in contacts", id));
    Ok(validate_email(email, contact))
}

fn validate_email(email: &str, contact: &Contact) -> String {
    if email.is_empty() {
        "Email cannot be empty".to_string()
    } else if email.to_string().contains('@') && email.to_string().contains('.') {
        if contact.email == email {
            "Same email as before".to_string()
        } else {
            "".to_string()
        }
    } else {
        "Invalid email".to_string()
    }
}

#[actix_web::main]
pub async fn main() -> std::io::Result<()> {
    let tera = Tera::new("./static/**/*.html").unwrap();
    let contacts_db = Arc::new(Mutex::new(instantiate_contacts()));
    let app_state = web::Data::new(AppState {
        contacts_vec: contacts_db.clone(),
        tera: tera.clone(),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(index)
            .service(contacts)
            .service(new_contact)
            .service(create_contact)
            .service(show_contact)
            .service(edit_contact)
            .service(update_contact)
            .service(delete_contact)
            .service(contacts_email_get)
            .service(contacts_count)
            .service(actix_files::Files::new("/static", "./static"))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

fn instantiate_contacts() -> Vec<Contact> {
    // 10 contacts
    vec![
        Contact {
            id: 1,
            first: "John".to_string(),
            last: "Doe".to_string(),
            phone: "555-1234".to_string(),
            email: "john@example.com".to_string(),
            error: None,
        },
        Contact {
            id: 2,
            first: "Jane".to_string(),
            last: "Doe".to_string(),
            phone: "555-4321".to_string(),
            email: "jane@example.com".to_string(),
            error: None,
        },
        Contact {
            id: 3,
            first: "Bob".to_string(),
            last: "Smith".to_string(),
            phone: "555-6789".to_string(),
            email: "bob@example.com".to_string(),
            error: None,
        },
        Contact {
            id: 4,
            first: "Alice".to_string(),
            last: "Brown".to_string(),
            phone: "555-9876".to_string(),
            email: "alice@example.com".to_string(),
            error: None,
        },
        Contact {
            id: 5,
            first: "Charlie".to_string(),
            last: "Lee".to_string(),
            phone: "555-4321".to_string(),
            email: "charlie@example.com".to_string(),
            error: None,
        },
        Contact {
            id: 6,
            first: "David".to_string(),
            last: "Williams".to_string(),
            phone: "555-6789".to_string(),
            email: "david@example.com".to_string(),
            error: None,
        },
        Contact {
            id: 7,
            first: "Emily".to_string(),
            last: "Brown".to_string(),
            phone: "555-7890".to_string(),
            email: "emily@example.com".to_string(),
            error: None,
        },
        Contact {
            id: 8,
            first: "Frank".to_string(),
            last: "Jones".to_string(),
            phone: "555-9012".to_string(),
            email: "frank@example.com".to_string(),
            error: None,
        },
        Contact {
            id: 9,
            first: "George".to_string(),
            last: "Smith".to_string(),
            phone: "555-3456".to_string(),
            email: "george@example.com".to_string(),
            error: None,
        },
        Contact {
            id: 10,
            first: "Hannah".to_string(),
            last: "Lee".to_string(),
            phone: "555-5678".to_string(),
            email: "hannah@example.com".to_string(),
            error: None,
        },
        Contact {
            id: 11,
            first: "Ivan".to_string(),
            last: "Ivanov".to_string(),
            phone: "555-1234".to_string(),
            email: "ivan@example.com".to_string(),
            error: None,
        },
    ]
}

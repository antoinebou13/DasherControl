use diesel::result::Error;
use rocket::http::{Status, Cookies, Cookie};
use rocket_contrib::json::Json;

use crate::db::DbConn;
use crate::tenants::token::*;
use crate::tenants::model::{AuthTenant, TenantInfo, RegisterTenantNoRole};
use crate::tenants::model::RegisterTenant;
use crate::tenants::model::Tenant;

#[get("/api/token")]
pub fn get_token(mut cookies: Cookies) -> Result<Json<String>, Json<String>> {
    let token_cookies = cookies.get_private("session-token");
    return match token_cookies {
        Some(c) => Ok(Json(c.value().to_string())),
        None => Err(Json("".to_string()))
    }
}

#[get("/api/list")]
pub fn all_tenants(conn: DbConn, token: Result<Claims, Status>) -> Result<Json<Vec<Tenant>>, Status> {
    let token = match token {
        Ok(token) => token,
        Err(e) => return Err(e)
    };
    return if token.has_role("Admin") {
        Tenant::all(&conn)
            .map_err(|error| error_status(error))
            .map(|tenants| Json(tenants))
    } else {
        Err(Status::Unauthorized)
    }
}

fn error_status(error: Error) -> Status {
    match error {
        Error::NotFound => Status::NotFound,
        _ => Status::InternalServerError
    }
}


#[post("/api/create", format="application/json", data = "<tenant>")]
pub fn create_tenant(conn: DbConn, tenant: Json<RegisterTenantNoRole>) -> Result<Json<String>, Status> {
    let register_tenant_no_role = tenant.into_inner();
    let register_tenant = RegisterTenant{
        email: register_tenant_no_role.email,
        name: register_tenant_no_role.name,
        username: register_tenant_no_role.username,
        role: "tenant".to_string(),
        password: register_tenant_no_role.password,
        password_confirmation: register_tenant_no_role.password_confirmation
    };
    let register_tenant = match register_tenant.validates(&conn) {
        Ok(register_tenant) => register_tenant,
        Err(_) => return Err(Status::Conflict),
    };
    return match Tenant::create(register_tenant, &conn) {
        Ok(_tenant) => Ok(Json("tenant created".to_string())),
        Err(_) => Err(Status::Conflict),
    }
}


#[post("/api/login", format="application/json", data = "<auth_tenant>")]
pub fn login(conn: DbConn, auth_tenant: Json<AuthTenant>, mut cookies: Cookies) -> Result<Json<TenantInfo>, Status> {
    let token_cookies = cookies.get_private("session-token");
    return if token_cookies.is_none() {
        let tenant = match auth_tenant.login(&conn) {
            Ok(tenant) => tenant,
            Err(_) => return Err(Status::Conflict),
        };


        // This is the jwt token we will send in a cookie.
        let token = create_token(
            tenant.id, &tenant.email, &tenant.username, &tenant.role, &tenant.login_session);

        cookies.add_private(
            Cookie::build("session-token", token)
                // .path("/")
                // .secure(true)
                .finish());

        Ok(Json(TenantInfo {
            email: tenant.email,
            username: tenant.username
        }))
    } else {
        let token = match token_cookies {
            Some(c) => c.value().to_string(),
            None => "".to_string()
        };
        return match decode_token(&*token) {
            Ok(claims) => Ok(Json(TenantInfo {
                        email: claims.email,
                        username: claims.username
                    })),
            Err(_) =>  Err(Status::Conflict),
        }
    }
}



#[post("/api/logout")]
pub fn logout(mut cookies: Cookies) -> Json<String> {
    if !cookies.get_private("session-token").is_none() {
        cookies.remove_private(Cookie::named("session-token"))
    }

    return Json("logout".to_string());
}





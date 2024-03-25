use pyo3::prelude::*;
use rnix::ast::{self, AttrpathValue, Entry, HasEntry};

#[pymodule]
fn nixserver_rust_bindings(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(enabled_services, m)?)?;
    Ok(())
}

/// Returns a list of services that are enabled in the given nix expression
#[pyfunction]
fn enabled_services(contents: &str) -> PyResult<Vec<String>> {
    let ast = rnix::Root::parse(&contents)
        .ok()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
    let expr = ast.expr().unwrap();
    let set = match recurse_to_attrset(expr) {
        Some(set) => set,
        None => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "no attribute set found".to_string(),
            ))
        }
    };
    let mut services = Vec::new();
    for entry in set.entries() {
        services.extend(handle_entry(entry, false));
    }
    Ok(services)
}

fn recurse_to_attrset(expr: ast::Expr) -> Option<ast::AttrSet> {
    match expr {
        ast::Expr::AttrSet(set) => Some(set),
        ast::Expr::LetIn(let_in) => let_in.body().and_then(|body| recurse_to_attrset(body)),
        ast::Expr::Lambda(ref lambda) => {
            if let Some(ast::Expr::AttrSet(set)) = lambda.body() {
                Some(set)
            } else {
                recurse_to_attrset(lambda.body().unwrap())
            }
        }
        _ => None,
    }
}

fn handle_entry(entry: Entry, is_service: bool) -> Vec<String> {
    let mut services = Vec::new();
    if let ast::Entry::AttrpathValue(attrpath_value) = entry {
        let value = attrpath_value.value();
        if let Some(ast::Expr::Ident(ident)) = value.clone() {
            services.extend(handle_ident(attrpath_value.clone(), ident));
        }
        if let Some(ast::Expr::AttrSet(attrset)) = value.clone() {
            services.extend(handle_attrset(attrpath_value.clone(), attrset, is_service));
        }
    }
    services
}

fn handle_ident(attrpath_value: AttrpathValue, ident: ast::Ident) -> Vec<String> {
    let mut services = Vec::new();
    // if the ident is not true, continue
    if ident.ident_token().unwrap().text() != "true" {
        return services;
    }

    let attrpath = attrpath_value.attrpath().unwrap();
    let mut attrs = attrpath.attrs();

    // check if the first attr is "services"
    if let Some(ast::Attr::Ident(ident)) = attrs.next() {
        if ident.ident_token().unwrap().text() != "services" {
            return services;
        }
    }

    let service = if let Some(ast::Attr::Ident(ident)) = attrs.next() {
        ident.ident_token().unwrap().text().to_string()
    } else {
        return services;
    };

    // check if the last attr is "enable"
    if let Some(ast::Attr::Ident(ident)) = attrs.next() {
        if ident.ident_token().unwrap().text() != "enable" {
            return services;
        }
    }

    services.push(service);
    services
}

fn handle_attrset(
    attrpath_value: AttrpathValue,
    attrset: ast::AttrSet,
    is_service: bool,
) -> Vec<String> {
    let mut services = Vec::new();

    let mut attrs = attrpath_value.attrpath().unwrap().attrs();

    if !is_service {
        // check if the first attr is "services" if not continue
        if let Some(ast::Attr::Ident(ident)) = attrs.next() {
            if ident.ident_token().unwrap().text() != "services" {
                return services;
            }
        } else {
            return services;
        }
    }

    let next_attr = attrs.next();

    if next_attr.is_none() {
        // recurse into the attrset
        for entry in attrset.entries() {
            services.extend(handle_entry(entry, true));
        }
        return services;
    }

    // check if the next attr is an ident/string
    let service = if let Some(ast::Attr::Ident(ident)) = next_attr {
        ident.ident_token().unwrap().text().to_string()
    } else {
        return services;
    };

    for apvs in attrset.attrpath_values() {
        // check if the first attr is "enable" if not continue
        let mut attrs = apvs.attrpath().unwrap().attrs();

        if let Some(ast::Attr::Ident(ident)) = attrs.next() {
            if ident.ident_token().unwrap().text() != "enable" {
                continue;
            }
        } else {
            continue;
        }
        // check if the value is true

        if let Some(ast::Expr::Ident(ident)) = apvs.value() {
            if ident.ident_token().unwrap().text() != "true" {
                continue;
            }
        }

        services.push(service.to_string());
    }
    services
}

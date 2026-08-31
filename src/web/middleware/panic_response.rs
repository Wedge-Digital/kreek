//! Ce que le journal doit dire quand un handler panique.
//!
//! `CatchPanicLayer::new()` journalise déjà — mais sur la cible
//! `tower_http::catch_panic`, et le filtre de l'application est
//! `kreek=<niveau>,sqlx=warn`. Une cible qui n'est ni `kreek` ni `sqlx` n'est
//! activée par aucune directive : **la ligne n'existe pas**. On aurait donc un
//! `500` propre et pas une ligne de journal — l'incident le moins renseigné
//! resterait le moins renseigné, ce que la carte 349 cherchait précisément à
//! corriger.
//!
//! C'est le même piège que la carte 344 avait trouvé sur le `TraceLayer`, qui
//! émettait sur `tower_http::trace` et se taisait pour la même raison. Il
//! reparaîtra à chaque couche tierce qu'on branchera en comptant sur sa
//! journalisation intégrée : **une bibliothèque journalise sur son propre nom,
//! et notre filtre ne connaît que le nôtre.**
//!
//! Émettre depuis ce module règle le problème par construction — la cible est
//! `kreek::web::middleware::panic_response` — et donne en prime un champ nommé
//! plutôt qu'un message formaté. La ligne hérite du span de requête, donc du
//! `rid`, du chemin et du coach, à condition que la couche reste posée **sous**
//! `request_log` dans `build_router`.

use axum::body::Body;
use axum::http::{header, HeaderValue, Response, StatusCode};
use std::any::Any;
use tower_http::catch_panic::ResponseForPanic;

/// Le corps de la réponse est du texte brut et non un fragment HTMX : un panic
/// n'est pas un cas métier, et rien ne garantit que l'état de l'application
/// permette encore de rendre quoi que ce soit de sensé.
#[derive(Debug, Default, Clone, Copy)]
pub struct JournalDePanic;

impl ResponseForPanic for JournalDePanic {
    type ResponseBody = Body;

    fn response_for_panic(&mut self, err: Box<dyn Any + Send + 'static>) -> Response<Body> {
        tracing::error!(
            panic = %message_du_panic(err.as_ref()),
            "panic dans un handler — requête abandonnée"
        );
        reponse_500()
    }
}

/// Deux `downcast` et non un : `panic!("texte")` produit un `&'static str`,
/// `panic!("{x}")` un `String`. N'en essayer qu'un fait perdre la moitié des
/// messages, et c'est toujours l'autre moitié qu'on cherche.
///
/// L'appelant doit passer `err.as_ref()` et **non** `&err` : `Box<dyn Any>`
/// est elle-même `Any`, donc `&err` produit un `&dyn Any` qui désigne la boîte
/// et non son contenu. Les deux `downcast` échouent alors systématiquement, et
/// tous les panics se journalisent « message illisible ». Le compilateur ne dit
/// rien — les deux formes typent.
fn message_du_panic(err: &(dyn Any + Send)) -> String {
    if let Some(s) = err.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = err.downcast_ref::<&str>() {
        (*s).to_string()
    } else {
        "message illisible — la charge du panic n'est ni String ni &str".to_string()
    }
}

fn reponse_500() -> Response<Body> {
    let mut reponse = Response::new(Body::from("Erreur interne du serveur"));
    *reponse.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
    reponse.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    reponse
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::services::observability::capture_journal::capture_sous_le_filtre_de_production;

    /// Le filtre réellement construit au démarrage, et non une chaîne recopiée :
    /// un test qui recopie le filtre continue de passer quand le vrai change.
    fn sous_le_filtre_de_production<T>(corps: impl FnOnce() -> T) -> (T, Vec<String>) {
        let (capture, _garde) = capture_sous_le_filtre_de_production();
        let resultat = corps();
        (resultat, capture.lignes())
    }

    #[test]
    fn la_ligne_de_panic_franchit_le_filtre_de_production() {
        let (_, lignes) = sous_le_filtre_de_production(|| {
            JournalDePanic.response_for_panic(Box::new("boum — panic volontaire de test"))
        });

        assert_eq!(lignes.len(), 1, "une ligne et une seule : {lignes:?}");
        assert!(
            lignes[0].contains("boum — panic volontaire de test"),
            "le message du panic doit être journalisé : {lignes:?}"
        );
    }

    /// Le test qui justifie l'existence de ce module. Il échouerait si
    /// quelqu'un revenait à `CatchPanicLayer::new()` en pensant que le
    /// gestionnaire par défaut journalise — il journalise, mais dans le vide.
    #[test]
    fn une_ligne_emise_sur_la_cible_de_tower_http_est_perdue() {
        let (_, lignes) = sous_le_filtre_de_production(|| {
            tracing::error!(target: "tower_http::catch_panic", "Service panicked: boum");
        });

        assert!(
            lignes.is_empty(),
            "le filtre `kreek=…` n'active aucune cible tierce : {lignes:?}"
        );
    }

    /// `panic!("texte")` produit un `&'static str`, `panic!("{x}")` un `String`.
    #[test]
    fn les_deux_formes_de_charge_de_panic_sont_lisibles() {
        assert_eq!(message_du_panic(&"litteral"), "litteral");
        assert_eq!(message_du_panic(&String::from("formate")), "formate");
        assert!(message_du_panic(&42_u8).contains("illisible"));
    }

    #[test]
    fn la_reponse_est_un_500_en_texte_brut() {
        let reponse = JournalDePanic.response_for_panic(Box::new("boum"));

        assert_eq!(reponse.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            reponse.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/plain; charset=utf-8"
        );
    }
}

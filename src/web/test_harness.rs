//! Harnais de test au niveau **handler** — le troisième étage.
//!
//! Entre le test unitaire (logique pure, co-localisé) et l'e2e (Playwright,
//! navigateur réel), il manquait de quoi exercer un handler Axum de bout en
//! bout sans navigateur. Ce manque a coûté trois reports de tests — cartes
//! 308, 315, puis les cinq cartes de cloisonnement des espaces.
//!
//! # Ce qu'il couvre, et ce qu'il ne couvre pas
//!
//! **Couvre** : statuts, en-têtes (`HX-Refresh`, `HX-Trigger`), matrices
//! d'autorisation. Sept endpoints × trois rôles, c'est vingt et une assertions
//! en millisecondes contre vingt et un scénarios de navigateur.
//!
//! **Ne couvre pas** : ni swap HTMX, ni Alpine, ni CSS. La règle de couverture
//! du projet existe parce que le bug du widget coach-search et celui des
//! pickers de tiers n'étaient visibles qu'en navigateur. Ce harnais **ne
//! remplace pas l'e2e** ; il évite d'y faire monter ce qui n'a rien à y faire.
//!
//! # Le câblage est celui de la production
//!
//! `compose()` et `build_router()` sont les fonctions que `main` appelle. Un
//! constructeur `for_tests` distinct aurait donné un harnais vert sur un
//! montage que la production n'a pas — c'est la seule chose qui rende ce
//! niveau de test digne de confiance.
//!
//! # L'identité passe par le vrai parcours de connexion
//!
//! Le harnais **se connecte** — `POST /auth/login`, puis il rejoue le cookie de
//! session sur chaque requête. Aucune ligne de production n'est ajoutée, et le
//! chemin d'authentification est exercé au passage.
//!
//! Deux voies ont été essayées avant. Élargir les profils de `bypass_auth`
//! aurait fait grossir du code de production pour les besoins des tests. Poser
//! une couche d'identité maison a **échoué** : appliquée par-dessus le routeur,
//! elle s'exécute *avant* `AuthManagerLayer` et ne peut pas extraire de session
//! — « Can't extract auth session ». Placer un middleware au bon endroit aurait
//! demandé de paramétrer `build_router`, c'est-à-dire d'y faire entrer les
//! tests.
//!
//! Le mot de passe est celui que `seed_e2e` pose sur tous ses comptes.

use crate::config::AppConfig;
use axum::body::Body;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::Router;
use tower::ServiceExt;

/// L'application sous test : le routeur de production, et le cookie de session
/// d'un utilisateur réellement connecté.
pub struct Harnais {
    routeur: Router,
    cookie: String,
}

impl Harnais {
    /// Monte l'application entière et s'y connecte sous le nom donné.
    ///
    /// `pool` vient de `#[sqlx::test]` : chaque test a sa base, donc les
    /// listeners lancés par `compose` n'interfèrent pas d'un test à l'autre.
    ///
    /// Le compte doit avoir été semé par `seed_e2e`, qui pose le même mot de
    /// passe sur tous les siens.
    pub async fn connecte_en_tant_que(pool: sqlx::PgPool, coach_name: &str) -> Self {
        let state = crate::compose(AppConfig::for_tests(), pool).await;
        let mut harnais = Self {
            routeur: crate::build_router(state),
            cookie: String::new(),
        };
        harnais.connecter(coach_name).await;
        harnais
    }

    async fn connecter(&mut self, coach_name: &str) {
        let corps = format!(
            "coach_name={coach_name}&password={}",
            crate::cli::seed_e2e::SEED_PASSWORD
        );
        let reponse = self
            .post_htmx(crate::app::auth::routes::path::LOGIN, &corps)
            .await;
        self.cookie = reponse
            .entete("set-cookie")
            .map(|c| c.split(';').next().unwrap_or_default().to_string())
            .unwrap_or_else(|| {
                panic!(
                    "connexion refusée pour « {coach_name} » : {}",
                    reponse.corps
                )
            });
    }

    pub async fn get(&self, uri: &str) -> Reponse {
        self.envoyer(
            Request::builder()
                .uri(uri)
                .header("cookie", &self.cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
    }

    /// `POST` de formulaire, avec l'en-tête `HX-Request` que le middleware CSRF
    /// exige.
    ///
    /// Encodé ici une fois pour toutes : sans lui, chaque test échouerait sur
    /// le CSRF, pour une raison étrangère à ce qu'il vérifie.
    pub async fn post_htmx(&self, uri: &str, corps: &str) -> Reponse {
        self.envoyer(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("HX-Request", "true")
                .header("cookie", &self.cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(corps.to_string()))
                .unwrap(),
        )
        .await
    }

    async fn envoyer(&self, requete: Request) -> Reponse {
        let reponse = self.routeur.clone().oneshot(requete).await.unwrap();
        let statut = reponse.status();
        let entetes = reponse.headers().clone();
        let corps = axum::body::to_bytes(reponse.into_body(), usize::MAX)
            .await
            .map(|o| String::from_utf8_lossy(&o).into_owned())
            .unwrap_or_default();
        Reponse {
            statut,
            entetes,
            corps,
        }
    }
}

pub struct Reponse {
    pub statut: StatusCode,
    pub entetes: axum::http::HeaderMap,
    pub corps: String,
}

impl Reponse {
    pub fn entete(&self, nom: &str) -> Option<&str> {
        self.entetes.get(nom).and_then(|v| v.to_str().ok())
    }
}

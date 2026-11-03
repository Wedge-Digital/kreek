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
    /// L'en-tête `Set-Cookie` **entier**, attributs compris.
    ///
    /// `cookie` n'en garde que `nom=valeur`, ce qu'il faut pour rejouer la
    /// session. Les attributs — `Max-Age`, `SameSite`, `Secure` — sont
    /// précisément ce que la carte 490 devait pouvoir vérifier : sans eux, un
    /// test ne distingue pas un cookie de session d'un cookie qui dure trente
    /// jours.
    set_cookie_brut: String,
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
        let mut harnais = Self {
            routeur: Self::mount(pool).await,
            cookie: String::new(),
            set_cookie_brut: String::new(),
        };
        harnais.connecter(coach_name).await;
        harnais
    }

    /// Le même navigateur après un redémarrage du serveur (carte 491).
    ///
    /// `compose`, le routeur **et le magasin de sessions** sont reconstruits —
    /// rien n'est repris de l'instance précédente que la base et le cookie.
    /// Réutiliser l'ancien magasin ferait passer n'importe quel magasin en
    /// mémoire pour persistant.
    pub async fn after_restart(self, pool: sqlx::PgPool) -> Self {
        Self {
            routeur: Self::mount(pool).await,
            ..self
        }
    }

    async fn mount(pool: sqlx::PgPool) -> Router {
        let sessions = crate::common::session_store::app_session_store(pool.clone());
        let state = crate::compose(AppConfig::for_tests(), pool).await;
        crate::build_router(state, sessions)
    }

    async fn connecter(&mut self, coach_name: &str) {
        let corps = format!(
            "coach_name={coach_name}&password={}",
            crate::cli::seed_e2e::SEED_PASSWORD
        );
        let reponse = self
            .post_htmx(crate::app::auth::routes::path::LOGIN, &corps)
            .await;
        self.set_cookie_brut = reponse.entete("set-cookie").unwrap_or_default().to_string();
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

    /// Le même routeur, présenté avec un autre cookie de session — celui d'un
    /// navigateur qui aurait volé une valeur ailleurs (carte 491).
    pub fn with_session_cookie(self, valeur: &str) -> Self {
        let nom = self.cookie.split('=').next().unwrap_or("id").to_string();
        Self {
            cookie: format!("{nom}={valeur}"),
            ..self
        }
    }

    /// La valeur du cookie de session, telle que le navigateur la renvoie.
    pub fn session_cookie_value(&self) -> &str {
        self.cookie
            .split_once('=')
            .map(|(_, v)| v)
            .unwrap_or_default()
    }

    /// L'en-tête `Set-Cookie` de la connexion, attributs compris.
    pub fn set_cookie_brut(&self) -> &str {
        &self.set_cookie_brut
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

    /// `GET` avec des en-têtes supplémentaires.
    ///
    /// Existe pour les fragments qui se situent depuis l'URL courante du
    /// navigateur plutôt que depuis leur propre chemin — le menu applicatif en
    /// est un : il déduit l'espace affiché de `hx-current-url`, et sans cet
    /// en-tête il ne rend qu'un menu vide, ce qui ferait passer n'importe quelle
    /// assertion d'absence.
    pub async fn get_avec(&self, uri: &str, entetes: &[(&str, &str)]) -> Reponse {
        let mut requete = Request::builder().uri(uri).header("cookie", &self.cookie);
        for (nom, valeur) in entetes {
            requete = requete.header(*nom, *valeur);
        }
        self.envoyer(requete.body(Body::empty()).unwrap()).await
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

    /// `GET` d'un flux SSE, connecté : statut, en-têtes et **premier bloc** du
    /// corps seulement.
    ///
    /// `envoyer` lit le corps jusqu'au bout, et un flux n'a pas de bout : le
    /// test attendrait indéfiniment (carte 642). Le corps est abandonné après le
    /// premier bloc, ce qui ferme le flux comme un onglet qu'on ferme.
    pub async fn open_stream(&self, uri: &str) -> StreamHead {
        let requete = Request::builder()
            .uri(uri)
            .header("cookie", &self.cookie)
            .body(Body::empty())
            .unwrap();
        self.stream_head(requete).await
    }

    /// Le même `GET`, sans cookie de session.
    pub async fn open_stream_anonymously(&self, uri: &str) -> StreamHead {
        let requete = Request::builder().uri(uri).body(Body::empty()).unwrap();
        self.stream_head(requete).await
    }

    async fn stream_head(&self, requete: Request) -> StreamHead {
        use futures_util::StreamExt;
        let reponse = self.routeur.clone().oneshot(requete).await.unwrap();
        let status = reponse.status();
        let headers = reponse.headers().clone();
        let mut corps = reponse.into_body().into_data_stream();
        let premier = tokio::time::timeout(std::time::Duration::from_secs(5), corps.next())
            .await
            .expect("aucun bloc dans les 5 s : le flux n'écrit rien");
        let first_chunk = premier
            .and_then(|bloc| bloc.ok())
            .map(|o| String::from_utf8_lossy(&o).into_owned());
        StreamHead {
            status,
            headers,
            first_chunk,
        }
    }
}

/// Le début d'une réponse en flux : ce qu'on peut lire sans attendre sa fin.
pub struct StreamHead {
    pub status: StatusCode,
    pub headers: axum::http::HeaderMap,
    /// `None` quand le corps est vide — une redirection, par exemple.
    pub first_chunk: Option<String>,
}

impl StreamHead {
    pub fn header(&self, nom: &str) -> Option<&str> {
        self.headers.get(nom).and_then(|v| v.to_str().ok())
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

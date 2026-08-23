# L'URL publique se recolle à la main, et interdit HTTPS

**Priorité : moyenne** — bloquant le jour d'un déploiement en HTTPS
**Trouvée par :** les cartes 339 et 340, qui ont ajouté trois occurrences de plus
**Fichiers :** `src/config.rs`, `src/main.rs`, `src/cli/send_notifications.rs`,
`src/app/auth/use_cases/send_reset_password_email.rs`

## Le problème

`host_domain` ne porte pas son schéma, et **cinq endroits** le lui recollent :

| | |
|---|---|
| `send_reset_password_email.rs:78` | l'URL de réinitialisation |
| `send_reset_password_email.rs:86` | `app_url` du logo (carte 325) |
| `cli/send_notifications.rs:81` | le cron (carte 340) |
| `main.rs:415` | le listener d'ouverture (carte 340) |
| `main.rs:451` | le contexte `competitions` (carte 339) |

Trois de ces cinq ont été ajoutées en deux jours, chacune en signalant la dette
sans la traiter. C'est le signe qu'il faut la traiter : une convention qu'on
recopie en s'excusant se recopiera encore.

**Poser `HOST_DOMAIN=https://kreek.example` produit aujourd'hui
`http://https://kreek.example/…`.** Le jour d'un déploiement en HTTPS, tous les
liens des e-mails sont cassés — et rien ne le signale avant qu'un coach clique.

## Les deux formes cohabitent déjà

`AppConfig::for_tests()` porte `"http://localhost"` — **avec** schéma — là où
`.env.dev` porte `localhost:3210` — **sans**. Aucune des deux n'est fautive
aujourd'hui puisque rien ne les confronte : le harnais ne recolle rien, le
serveur recolle toujours.

`.env.example` et `.env.test` posent par ailleurs `HOST_DOMAIN=` **vide**, ce qui
ne renseigne personne sur la forme attendue et produit des liens `http:///app/…`
si quelqu'un s'en sert tel quel.

Rien de tout cela n'est un défaut vivant — c'est du terrain meuble sous celui
qui l'est.

## Conception

### Une méthode qui normalise, plutôt qu'une forme imposée

`AppConfig::app_url()` rend `host_domain` tel quel s'il porte déjà un schéma, et
lui préfixe `http://` sinon. Les cinq usages l'appellent ; plus personne ne
recolle quoi que ce soit.

**Pourquoi tolérer les deux formes.** Imposer `HOST_DOMAIN=https://…` serait plus
propre, mais exigerait de coordonner la mise à jour de chaque déploiement au
moment même du déploiement — et un `.env` oublié produirait des liens cassés
sans rien signaler. La normalisation marche dans les deux cas, avant et après.

En contrepartie, la **forme documentée** devient celle avec schéma : le défaut du
code et les fichiers d'exemple s'alignent dessus.

### Ce à quoi on ne touche pas

`AuthContext` porte `host_domain` alors que seul l'e-mail s'en sert. C'est un
**BC extractible** : lui faire lire `AppConfig` créerait exactement l'adhérence
que l'axe 9 proscrit. L'hôte continue de lui injecter une chaîne — simplement,
c'est désormais l'URL complète.

## Checklist

- [x] `AppConfig::app_url()`, et les cinq usages qui l'appellent
- [x] `for_tests()`, `.env.dev`, `.env.example` et `.env.test` alignés sur la
      forme avec schéma
- [x] Renommer `AuthContext.host_domain` → `app_url` : le champ porte désormais
      une URL complète, et son ancien nom invite à lui recoller un schéma
- [x] Tests : sans schéma, `http://`, `https://`, barre finale, harnais
- [x] Un test refuse tout nouveau `format!("http://` recollé à `host_domain` —
      ce qui a été recopié trois fois se recopiera une quatrième
- [x] `make check-arch`, `make test`

## Ce que la carte ne couvre pas

Le port et le chemin de base. `host_domain` mêle hôte et port (`localhost:3210`)
et c'est très bien ainsi ; le découper n'apporterait rien tant qu'aucun
déploiement ne sert l'application sous un sous-chemin.

# Harnais de test au niveau handler

**Priorité : moyenne**
**À faire après :** `308-players-customisation-endpoints.md`
**Contexte :** transverse — `main.rs`, `state.rs`, support de test

## Le manque

Le projet n'a **aucun test au niveau handler**. Entre le test unitaire (logique
pure, co-localisé) et l'e2e (Playwright, navigateur réel), il n'existe rien qui
exerce un handler Axum de bout en bout sans navigateur.

Conséquence concrète, découverte en écrivant la carte 308 : ses trois
vérifications de contrat HTTP — « membre simple → 403 sur chacun des sept
endpoints », « refus métier → 200 et panier intact », « version périmée →
panneau re-rendu sans message » — n'ont pas d'endroit où vivre. Elles ne sont
pas de la logique pure, et les faire en navigateur coûte sept scénarios
Playwright pour ce qui tient en sept assertions en mémoire.

## Ce qui est déjà en place

L'architecture a posé les coutures ; il manque un point d'assemblage.

- L'injection est **manuelle et explicite** : chaque contexte de BC ne reçoit
  que des `Arc<dyn Trait>`. Un état de test se compose sans rien inventer.
- Les **ports inter-BC sont des traits**, donc doublables. La règle « ne pas
  mocker sqlx » vise les repositories, pas les ports d'ACL.
- Les tests d'intégration tournent déjà sur une **vraie base** via
  `#[sqlx::test]`.

## Les quatre obstacles, par coût décroissant

### 1. Construire un `AppState` — le vrai travail

Il n'est assemblé que dans `main.rs`, en dur. Tout le reste en découle.

**Le risque à ne pas manquer** : un constructeur `for_tests` qui diverge de
`main.rs` donne un harnais vert sur un câblage que la production n'a pas.
La parade est une contrainte, pas une convention : **extraire la composition
dans une fonction unique**, appelée par `main.rs` *et* par les tests. C'est un
gain propre pour `main.rs` indépendamment des tests.

### 2. L'identité — la seule vraie décision

`bypass_auth` existe et sait choisir un profil par en-tête, mais il n'en
connaît que deux : `DevCoach` (`legacy_id = 1`) et le membre simple. La 308 a
besoin de trois rôles distincts — admin de compétition **qui n'est pas le
coach**, coach de l'équipe, tiers.

Deux voies :

- **élargir les profils de `bypass_auth`** — cohérent avec l'existant, mais on
  continue d'empiler des cas de test dans du code de production, dans un
  `match` que personne ne saura plus élaguer ;
- **un middleware de test** qui pose un utilisateur arbitraire en session,
  réservé au harnais.

Penchant : la seconde. `bypass_auth` sert un usage de développement, le harnais
un usage de test ; les mélanger fait grossir le mauvais fichier.

### 3. Le CSRF

Un `POST` sans `HX-Request: true` est rejeté. Trivial — mais à encoder dans un
helper, sinon chaque test échoue pour une raison étrangère à ce qu'il vérifie.

### 4. La mécanique

`oneshot` sur le routeur composé, `to_bytes` sur la réponse. Partie gratuite.

## Ce que ça achète — et ce que ça n'achète pas

**Achète** : statuts, en-têtes (`HX-Refresh`, `HX-Trigger`), matrices
d'autorisation. Sept endpoints × trois rôles, c'est 21 assertions en
millisecondes contre 21 scénarios navigateur.

**N'achète pas** : ni swap HTMX, ni Alpine, ni CSS. La règle de couverture du
projet existe précisément parce que le bug du widget coach-search et celui des
pickers de tiers n'étaient visibles qu'en navigateur.

**Ce serait un troisième étage, pas un remplacement.** À documenter comme tel
dans le `CLAUDE.md`, sinon il servira d'excuse pour sauter l'e2e.

Corollaire mesuré (cf. carte sœur sur les temps d'exécution) : ce harnais
**n'allège presque rien de l'existant**. Les contrats HTTP ne sont aujourd'hui
pas testés du tout. Sa valeur est d'empêcher la suite e2e de grossir quand on
les ajoutera, pas de la raccourcir.

## Ordre de réalisation

1. Extraire la composition de l'`AppState` de `main.rs` — sans changement de
   comportement, vérifiable immédiatement
2. Helpers de requête : construction du routeur, `get`, `post_htmx`, corps →
   chaîne
3. Trancher l'identité (§2)
4. **Prouver sur un seul cas** : la matrice 403 de la carte 308 — meilleur
   rapport valeur/effort du projet
5. Seulement ensuite : l'inscrire dans le `CLAUDE.md` comme tier à part entière

## Points ouverts

- **Emplacement des tests.** Le projet co-localise (`io/repository/tests/`) ;
  `src/app/<bc>/io/web/tests/` suivrait la convention.
- **`check-arch` axe 8** exige qu'un test e2e figure dans la carte d'impact.
  Ce tier n'en relève pas — il tourne dans `make test`. À confirmer que l'axe
  ne s'y applique pas par accident de nommage.
- **Ce qui n'a pas besoin du harnais.** Une part de ce que la 308 vérifie se
  teste en fonctions pures, comme `choose_right_panel` en carte 307. Le
  harnais ne se justifie que pour ce qui **est** la composition : autorisation
  et en-têtes.

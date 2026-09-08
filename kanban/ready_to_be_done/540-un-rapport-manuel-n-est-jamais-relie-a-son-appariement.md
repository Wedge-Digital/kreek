# Un rapport manuel n'est jamais relié à son appariement

**Priorité : haute — défaut de production, constaté sur données réelles**
**Dépend de :** rien
**Épic :** E17 — Corriger le calendrier sans perdre la saisie
**Trouvé par :** l'enquête sur l'incident G. B. L. R du 2026-09-07
**Fichiers :** `src/app/match_report/use_cases/create_match_report_use_case.rs`,
`src/app/match_report/domain/match_report_draft.rs`,
migration de données pour les rapports déjà orphelins

## Le défaut

Quand un coach saisit un match hors calendrier :

1. un rapport naît, `origin: Manual`, **`pairing_id: NULL`** ;
2. `competitions/io/app_events/appariement.rs` fabrique un appariement pour lui
   et émet `PairingCreated` ;
3. `match_report/io/app_events/pairing_created_listener.rs` réagit et appelle
   `create_match_report_use_case` **avec le `pairing_id`** ;
4. ce use case commence par `find_id_by_round_and_teams`, retrouve le rapport
   manuel existant et le **confirme** au lieu d'en créer un second — et
   `confirm_existing` **laisse tomber le `pairing_id` qu'il transportait**.

Le rapport reste donc à `pairing_id = NULL` alors qu'un appariement le désigne.

## Ce que ça produit

```rust
// pairing_deleted_listener.rs
let mr_id = match repo.find_id_by_pairing(&pairing_id).await {
    Ok(None) => return,   // ← ici, pour tout rapport manuel
```

**Supprimer l'appariement retire la rencontre du calendrier et laisse le rapport
vivant**, orphelin et invisible. L'administrateur doit faire un second geste que
rien ne lui annonce.

Pire : la garde qui refuse de supprimer un appariement dont le rapport est
publié passe par le même lien. Sans lien, **elle ne s'applique pas non plus** —
un match publié peut disparaître du calendrier en continuant de compter au
classement. Le listener journalise ce cas comme une erreur, mais il n'est jamais
atteint.

## Constaté en production

Espace G. B. L. R, 7 septembre. Rapport `01M1Y343FY…`, `origin: Manual`,
`pairing_id: null`, pour l'appariement `01M1Y343J73…` créé à 14:07:30 et
supprimé à 19:50:29. Le rapport a survécu treize minutes de plus, jusqu'à une
annulation manuelle. Les identifiants partagent leur préfixe ULID : ils sont nés
dans la même milliseconde.

## Conception

**Le correctif tient en une écriture.** `confirm_existing` doit poser le
`pairing_id` reçu quand le rapport n'en a pas — un événement de domaine, sa
transition, la projection.

Ne **jamais écraser** un `pairing_id` déjà posé : deux appariements pour une
même affiche existent (c'est l'incident), et le premier lien est le bon.

### Les orphelins existants

Le correctif répare l'avenir. Les rapports manuels déjà créés restent sans lien,
et une migration de données les rattache par `(round_id, équipes)` — la même
clé que `find_id_by_round_and_teams`. La table `applied_data_migrations` existe
pour ça ; **aucune sous-commande CLI ne l'exploite encore**, ce qui est à
décider dans cette carte.

Compter d'abord : `SELECT count(*) FROM match_report_proj WHERE pairing_id IS
NULL AND origin = 'Manual'` dit l'ampleur avant qu'on écrive quoi que ce soit.

## Checklist

- [ ] Compter les orphelins en base avant de coder
- [ ] L'événement de rattachement, sa transition, la projection
- [ ] `confirm_existing` pose le lien, et n'écrase jamais un lien existant
- [ ] Test : un rapport manuel confirmé par le listener porte son `pairing_id`
- [ ] Test : supprimer l'appariement d'un rapport manuel **annule** ce rapport
- [ ] Test : la garde du rapport publié s'applique désormais aussi à eux
- [ ] La migration de données, ou la décision écrite de ne pas la faire
- [ ] `make lint`, `make check-arch`, `make test`

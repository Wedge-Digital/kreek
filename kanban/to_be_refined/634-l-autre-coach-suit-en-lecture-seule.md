# L'autre coach suit en lecture seule

**Épic :** E21 — Le mode de saisie et l'après-match en séquence · **Ordre :** 4 · **Dépend de :** 631
**Conception :** `docs/specs/saisie-mobile/E21-cadrage.md` (R5, § 4, § 8.7, § 8.8)

## Objectif

En « Je saisis tout », **seuls le démarreur et les admins écrivent** : saisie,
publication, dépublication, correction. Aujourd'hui, la garde de E19
(`io/web/garde_saisie.rs`) laisse écrire les deux coachs, et la publication
comme la dépublication ne demandent que `is_authorized`
(`io/web/recap_controller.rs:463-474`).

## Conception

**Le prédicat**, à côté d'`is_authorized` dans `use_cases/match_report_access_service.rs` :

```rust
// arch:no-instrument — service de lecture : une question de droit, aucune intention métier
pub async fn can_write_report(deps: &AccesRapportDeps<'_>, user: &User, space_id: &str,
    scope: &PorteeRapport, entry: &EntryScope) -> bool;
```

Admin (`est_administrateur`) → oui. Brouillon (`started_by` vide) → tout
autorisé peut le démarrer. `SoloEntry` → `user.id == started_by`. Il se lit sur
la projection (631), comme la portée de E19.

**Le routeur** passe à trois groupes (`router.rs`) :

| Groupe | Garde |
|---|---|
| `routes_ouvertes()` | inchangé |
| `routes_de_suivi()` — nouveau, vide jusqu'à 635 et 636 | `is_authorized` |
| `routes_de_saisie()` | `is_authorized` puis `can_write_report` |

Refus de `can_write_report` : écriture ou requête HTMX → **403** ; page
complète → une page lisible, « Ce match est saisi par *Yann*. » (décision du
2026-11-01), que 636 remplacera par la redirection vers `/live`.

**Le récapitulatif** : `authorize_recap_action` exige aussi `can_write_report`
pour publier et dépublier ; le bouton de publication et la zone de correction
ne s'affichent que pour qui le passe. Le récapitulatif lui-même reste visible
aux deux coachs.

## Données de production

Aucune donnée modifiée. **Acceptée** (2026-11-01) : dans les comptes rendus en
cours à la livraison, l'autre coach que le démarreur est bloqué, même s'il
saisissait ; un compte rendu publié ne se corrige plus que par son démarreur ou
un admin.

**En attente de décision** (§ 8.8) : le cas « démarreur = admin qui ne coache
aucune des deux équipes » n'est pas tranché. **Mesure sur dump avant
livraison** : les rapports en cours dont le démarreur
n'est coach d'aucune des deux équipes — un admin a confirmé pour eux, et **les
deux** coachs y seraient bloqués. Le décompte est soumis à l'utilisateur, et
la carte ne se livre pas avant sa décision.

## Checklist

- [ ] `can_write_report` et sa lecture sur la projection
- [ ] Trois groupes de routes ; le refus en 403 ou en page lisible
- [ ] Publication, dépublication, correction : `can_write_report` ; boutons masqués
- [ ] Tests unitaires : démarreur, admin non démarreur, autre coach, tiers, brouillon
- [ ] E2E : `test_match_report_access.py` — `test_le_coach_d_une_equipe_du_match_y_entre` migré (le membre simple n'a pas démarré) ; l'autre coach reçoit 403 sur une action et sur la publication
- [ ] Mesure sur dump consignée, décision de l'utilisateur notée
- [ ] `tests/impact-map.toml` mis à jour
- [ ] `make lint`, `make check-arch`, `make test`, e2e impactés

# La session ne glisse pas : elle expire trente jours après la connexion

**Épic :** aucune · **Dépend de :** 491 (le magasin persistant)
**Priorité : moyenne** — un coach actif est déconnecté au trentième jour
**Trouvée par :** le plan de la carte 491 (2026-11-02). Constat de lecture du
code de `tower-sessions`, **à vérifier par un test** avant toute correction.

## Le constat

La carte 490 a posé `Expiry::OnInactivity(30 jours)` (`src/main.rs:682`), et le
commentaire de `main.rs:670` annonce que la fenêtre « glisse à chaque requête ».

D'après le plan de la 491, `tower-sessions` 0.14 (`service.rs:251`) ne réécrit
l'enregistrement et le cookie que si la session est **modifiée**, ou si
`always_save` vaut `true`. Rien ne modifie la session après la connexion : elle
expire donc trente jours après la connexion, pas après la dernière visite.

La 491 corrige seulement le commentaire ; elle s'interdit de toucher à la durée.

## Ce qu'il faut faire

1. **Le prouver** : un test par le harnais qui avance le temps (ou lit
   `Max-Age`/`expiry_date` sur deux requêtes espacées) et constate que
   l'échéance ne bouge pas.
2. **Corriger sans écrire à chaque requête.** `with_always_save(true)` réglerait
   le défaut au prix d'un UPSERT par requête authentifiée, fragments htmx
   compris. Préférer une réécriture seulement quand l'échéance approche — par
   exemple quand il reste moins de 29 jours.
3. Remettre `CLAUDE.md` (« Sessions ») d'accord avec ce qui se passe.

## Terminé quand

- [ ] un test montre qu'une requête faite au jour J repousse l'échéance à
      J + 30 ;
- [ ] le nombre d'écritures dans `sessions` reste d'au plus une par session et
      par jour.

## Conséquences sur les données de production

Aucune donnée nouvelle : la table `sessions` de la 491 reçoit quelques mises à
jour de plus, au plus une par session et par jour.

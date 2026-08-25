# Saisie des actions — gain de la Haine · Phase 8 : cartes kanban

**Entrée** : phases 2 à 7 validées. Dernière phase de conception.

## Les six cartes

Ordonnées par dépendance — domaine, puis use case, puis handler, puis template.
Chacune compile, se teste et se commite seule.

| # | Carte | Livrable observable |
|---|---|---|
| **399** | Le corpus connaît les mots-clefs | `keywords_fr.json` chargé, `keywords` sur les lignes de roster, les 38 `HAINE_<UID>` en catégorie `TRAITS` |
| **400** | Le domaine sait refuser une Haine | `HatredKeyword`, `peut_donner_haine()`, `Blesse { hatred }`, `record_action` → `Result` |
| **401** | Enregistrer une Haine par POST | port du catalogue, use case, les trois refus en 422 — sans écran |
| **402** | La section dans le panneau d'action | domain service, widget, template, CSS, journal des actions |
| **403** | La Haine atteint le joueur | app event, publisher, `AcquisitionMode::Injury`, `record_hatred`, projection |
| **404** | La Haine sous Playwright | les huit scénarios |

```
399 ──► 400 ──► 401 ──┬──► 402 ──┐
                      └──► 403 ──┴──► 404
```

**402 et 403 sont parallélisables** : l'écran et la traversée vers `players` ne
se touchent pas. 404 attend les deux.

## Trois écarts assumés à la règle de découpage

**La 400 embarque l'adaptation du use case.** `record_action` passe de couple à
`Result` : sans propager immédiatement, le projet ne compile plus. Le prix d'une
signature qui change se paie dans la carte qui la change, pas dans la suivante.

**La 402 fusionne le panneau et le journal des actions.** La règle veut une carte
par widget, mais le journal ne représente que l'affichage de « + Haine : Nain »
sur une ligne existante. Cinq lignes de template ne valent pas une carte à elles.

**Aucune carte pour l'affichage sur la fiche joueur.** La Haine entrant dans
`acquired_skills`, le template la rend déjà. Le badge violet des traits et les
mots-clefs du poste appartiennent aux **deux autres pages** de cette
fonctionnalité, qui n'ont pas encore leurs phases 2 à 7.

## Ce qui reste hors de ce lot

- **La fiche d'équipe** — mots-clefs sous le poste, en italique.
- **La fiche joueur** — mots-clefs dans l'en-tête, badge des traits, journal des
  évolutions affichant « Blessure ».
- **L'effet de la Haine en jeu.** Les lignes de roster portent leurs `keywords`,
  ce qui rend la comparaison possible ; aucune règle d'effet n'est spécifiée.

## La dépendance qui n'est pas dans les cartes

Le **corpus de production** devra porter `keywords_fr.json` et les trente-huit
compétences `HAINE_<UID>` avant tout déploiement. La 399 ne fournit que le jeu de
démonstration ; elle pose en revanche la garde qui fait échouer bruyamment un
corpus muet, plutôt que d'afficher un sélecteur vide.

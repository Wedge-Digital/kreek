# Les comptes d'expédition ne vont nulle part

**Priorité : moyenne — une fonction qui marche et ne le dit pas**
**Épic :** E16 — Sondage de présence
**Dépend de :** 527 (qui produit les trois compteurs)
**Fichiers :** `src/app/competitions/io/web/admin/presences_actions.rs`,
`.../presences_widgets.rs`, un gabarit de bandeau

## Ce qu'on a découvert

`LaunchOutcome` porte `destinataires`, `sans_adresse`, `envoyes`,
`deja_envoyes` et `echecs`. `RemindOutcome` porte `relances`, `envoyes`,
`deja_envoyes` et `echecs`. **Aucun n'atteint l'écran** :

```rust
match issue {
    Ok(_) => succes(),          // presences_actions.rs — les compteurs tombent ici
```

`succes()` rend un corps vide avec `HX-Trigger: presenceChanged`, les deux
widgets se rechargent, et le panneau relit la campagne — qui ne stocke aucun de
ces nombres. Ils sont calculés, journalisés, puis perdus.

## Pourquoi ça compte

R20 dit qu'un échec d'envoi est **journalisé, pas propagé**, et la carte 515
écrivait que `echecs` « est un fait que l'écran annonce ». L'écran n'annonce
rien. Concrètement : un organisateur dont le serveur de messagerie est en panne
lance sa campagne, voit le panneau se recharger normalement, et **croit ses
quatorze e-mails partis**. Il l'apprendra par un coach, ou pas du tout.

`deja_envoyes` est le cas le plus parlant : il ne peut se produire que sur un
double clic ou une relance du même jour, c'est-à-dire exactement quand
l'organisateur se demande si son action a été prise en compte.

Le compte `sans_adresse` (R3), lui, **est** affiché — mais avant le lancement,
dans le panneau de préparation. Après, plus rien.

## Piste, à discuter

Un bandeau rendu par la réponse du POST, plutôt qu'un corps vide : « 12 e-mails
envoyés · 2 coachs sans adresse · 1 échec ». Il faut alors décider s'il remplace
`succes()` pour les dix actions ou seulement pour les deux qui expédient, et
comment il cohabite avec `HX-Trigger: presenceChanged` — le fragment de succès
ne vise aucune cible aujourd'hui.

## Questions ouvertes

- Un bandeau éphémère (toast) ou une ligne persistante dans le panneau ?
- Que montre-t-on quand tout s'est bien passé — rien, ou « 12 envoyés » ?
- Faut-il **persister** le dernier rapport d'expédition sur la campagne ? Sans
  ça, un rechargement de page le perd, et la question « est-ce parti ? » se
  reposera le lendemain.

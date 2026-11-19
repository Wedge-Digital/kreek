# Le widget de dépense de SPP retombe en silence sur le journal

> **⚠️ Cette carte demande ton attention.** Le comportement actuel est peut-être
> délibéré — la question est de savoir si un refus doit rester indiscernable
> d'un succès.

**Priorité : basse** — rien n'est cassé, mais toute panne de ce widget est
illisible
**Dépend de :** rien
**Trouvée par :** le diagnostic de `test_player_spp_spending.py`, où ce repli a
été la première fausse piste

## Le constat

`src/app/players/io/web/widgets/spp_spending_widget.rs` :

```rust
if !is_eligible(&state, &user, &space_id, &player).await {
    // Défense en profondeur : si l'URL est atteinte directement hors
    // contexte éligible, on retombe sur le journal en lecture seule.
    return evolution_journal_widget(...).await.into_response();
}
```

Le refus rend **200 OK** avec le contenu d'un autre widget. Du point de vue du
navigateur, la demande a réussi ; du point de vue de l'utilisateur, le clic sur
« Activer la dépense de SPP » n'a rien fait.

L'intention — ne pas ouvrir la dépense hors phase — est juste. C'est la **forme
du refus** qui pose problème.

## Ce que ça a coûté

Pendant le diagnostic d'un test instable, ce repli était l'explication la plus
plausible : un panneau inchangé après un clic est exactement ce qu'il produit.
Il a fallu vérifier le défaut de `EvolutionJournalParams::can_spend`
(`false`, donc le repli aurait affiché le cadenas et non le bouton) pour
l'écarter, puis enregistrer le trafic réseau pour établir qu'**aucune requête
n'était partie**. La vraie cause était ailleurs.

Un refus qui ressemble à un succès coûte ce genre de détour à chaque
investigation.

## Ce qui est à trancher

**Répondre un statut qui dit non.** Un 409 ou un 403 laisserait htmx ne rien
remplacer — mais c'est déjà ce qui se passe visuellement, sans que le serveur
l'ait dit.

**Rendre le journal avec son cadenas**, c'est-à-dire passer `can_spend: false`
au lieu du `default()` actuel : l'écran expliquerait alors pourquoi la dépense
n'est pas ouverte, au lieu de réafficher le bouton qui vient d'échouer.

**Journaliser le refus.** Aujourd'hui il ne laisse aucune trace. Une ligne sous
une cible `kreek::` suffirait à ce qu'un `grep rid=` retrouve la cause — c'est
la règle d'observabilité du projet, et ce chemin y échappe.

**Ne rien changer**, en documentant que le repli est volontaire.

## Ce que la carte ne couvre pas

**L'instabilité du test**, corrigée par ailleurs : le clic partait avant le
câblage htmx du bouton, et le repli n'y était pour rien.

**La logique d'éligibilité elle-même** (`in_player_improvement_phase` et
`can_spend_spp`), qui n'est pas en cause.

## Questions à trancher au raffinement

- Ce repli a-t-il déjà servi en conditions réelles, ou est-ce une défense qui
  n'a jamais eu à jouer ?
- Le même motif « refuser en rendant autre chose avec un 200 » existe-t-il
  ailleurs dans `players` ou dans d'autres BCs ?

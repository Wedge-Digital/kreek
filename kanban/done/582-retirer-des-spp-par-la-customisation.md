# Retirer des SPP par la customisation

**Priorité : moyenne**
**Épic :** aucune
**Dépend de :** rien
**Fichiers :** `src/app/players/domain/value_objects.rs`,
`src/app/players/domain/player.rs`, `src/app/players/domain/customisations.rs`,
`src/app/players/domain/customisation_basket.rs`, `src/app/players/domain/error.rs`,
`src/app/players/io/repository/player_repository.rs`,
`src/app/players/io/web/widgets/player_customisation_widget.rs`,
`src/app/players/io/web/templates/player-customisation-widget.html`,
`src/app/players/io/web/widgets/evolution_journal_widget.rs`,
`tests/e2e/test_player_customisation.py`

## L'objectif

L'onglet SPP de la customisation ne sait qu'**ajouter**. Un commissaire doit
pouvoir aussi **retirer** des SPP à un joueur — une erreur de saisie, une
sanction.

## Les décisions (2026-10-03)

1. **On ne retire que des SPP non dépensés** : au plus `spp_remaining()`.
   Retirer des SPP déjà dépensés obligerait à défaire des achats — un autre
   chantier.
2. **Un montant signé**, sur le modèle du prix (`KpoDelta`) : `SppAmount`
   devient `SppDelta`, et `PlayerSppCustomised` porte un delta. Les événements
   persistés, tous positifs, se relisent tels quels.
3. **De −100 à +100 par opération**, non nul — le miroir du plafond actuel.

## Le changement

- **Domaine** : `SppDelta` (signé, non nul, de −100 à 100) remplace `SppAmount`.
  `customise_spp` refuse un retrait supérieur au disponible ; le panier rejoue
  la garde à la validation, contre l'état accumulé des lignes précédentes.
  `apply()` additionne le delta.
- **Retrait d'une customisation**, symétrique : retirer un **ajout** garde la
  garde existante (`SppDepenses`) ; retirer un **retrait** rend les SPP, sans
  garde.
- **Projection** : `spp = spp + $2` accepte déjà un négatif ; le retrait de la
  projection suit la symétrie.
- **Écran** : champ signé comme celui du prix (« Ex. 5 ou -3 »), bouton
  « Appliquer », motif de refus quand le retrait dépasse le disponible ; le
  signe se lit dans le panier et la liste des customisations appliquées.
- **Journal d'évolution** : « −5 SPP » pour un retrait.

## Tests

Unitaires : la garde du retrait ; le rejeu du panier (deux retraits qui
dépassent ensemble le disponible) ; la symétrie du retrait d'une customisation ;
la relecture d'un événement positif déjà persisté.

E2E : retirer des SPP ; un retrait au-delà du disponible est refusé ; retirer
un retrait rend les SPP.

## Terminé quand

Un commissaire retire des SPP non dépensés à l'écran, ne peut pas en retirer
plus, et la suite complète passe.

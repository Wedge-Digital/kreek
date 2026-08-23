# Page hôte + onglet Membres — Cartes kanban

**Entrée :** phases 2 à 7 validées. Dernière phase de conception.

## Les onze cartes

| # | Carte | Apport |
|---|---|---|
| 364 | Deux événements de `spaces` portent le même type | corrige un défaut latent, **préalable à tout** |
| 375 | L'agrégat `Space` se charge incomplet, et hors de sa souveraineté | second préalable — sans elle l'invariant **répond faux** |
| 365 | L'appartenance à un espace devient un invariant | agrégat, value object, erreurs, événements |
| 366 | Le dépôt sait lire, modifier et retirer une appartenance | trois méthodes, trois fichiers SQL |
| 367 | Changer le rôle et retirer un membre, côté applicatif | deux use cases |
| 368 | La page d'administration et sa barre d'onglets | le cadre, la garde, la réservation de hauteur |
| 369 | La liste des membres | widget de lecture, VMs, fragment de ligne |
| 370 | Les statistiques de l'espace | widget de lecture, trois compteurs |
| 371 | Changer un rôle, retirer un membre, depuis l'écran | deux actions, re-rendu de ligne |
| 372 | Le bouton de réinitialisation de mot de passe | route `auth`, injection par l'hôte |
| 373 | `competitions` réagit au retrait d'un membre | listener cross-BC |
| 374 | La page d'administration sous Playwright | couverture e2e |

## Ce qui commande l'ordre

**364 d'abord, et seule.** `USER_INVITED_IN_SPACE` et
`USER_SUBSCRIBED_TO_SPACE` valent la même chaîne. La carte 365 touche ce
fichier ; corriger en même temps mêlerait un défaut préexistant à une
fonctionnalité, dans un commit où plus personne ne saurait lequel des deux a
cassé quoi.

**375 avant 365.** L'agrégat n'est chargé par personne aujourd'hui, et deux
défauts y dorment : un membre sans avatar en est silencieusement absent, et le
SQL joint `auth__users`. Ce n'est pas un nettoyage adjacent — sans elle,
l'invariant du dernier administrateur ne tombe pas en panne, il **répond faux**.

**365 → 366 → 367** est une chaîne de dépendances stricte : le use case appelle
l'agrégat et le dépôt, qui n'existent pas avant.

**368 avant 369 et 370** — les deux widgets n'ont pas de page où s'afficher
avant elle. En revanche **369 et 370 sont parallèles** : deux endpoints
distincts, deux feuilles distinctes, aucun lien entre eux.

**371 après 367 et 369** — les actions appellent les use cases et re-rendent le
fragment de ligne que 369 crée.

**372 est indépendante de tout le reste.** Elle ne touche ni l'agrégat, ni les
use cases : une route dans `auth`, une méthode dans `ISpacesHostLayout`, un
bouton. Elle peut se faire à n'importe quel moment après 369.

**373 après 367**, qui fait émettre l'app event. Elle est dans un autre BC et se
teste autrement — intégration sur `competitions_members`, pas agrégat.

**374 en dernier**, quand il y a quelque chose à piloter.

```
364 ──► 365 ──► 366 ──► 367 ──┬──► 371 ──┐
                              │          │
                              └──► 373   │
                                         │
        368 ──┬──► 369 ────────────────►─┤──► 374
              └──► 370                   │
                                         │
        372 ─────────────────────────────┘
```

## Ce que les cartes ne redisent pas

La conception vit dans les fichiers de spec de ce dossier. Chaque carte y
renvoie plutôt que de la recopier : une carte qui duplique sa spec en devient la
seconde version, et les deux divergent au premier ajustement.

## Ce qui reste hors de ces onze cartes

- **Les trois autres onglets** — Ajout direct, Invitations, Paramètres. Chacun
  reprendra le workflow à la phase 2.
- **Le badge de visibilité de la bannière**, qui arrive avec l'onglet
  Paramètres — décidé en phase 4.
- **La zone de danger** — épic dédiée, décidée en phase 1.

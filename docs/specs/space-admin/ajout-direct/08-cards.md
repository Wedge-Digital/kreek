# Onglet Ajout direct — Cartes kanban

**Entrée :** phases 2 à 7 validées. Dernière phase de conception.

## Les neuf cartes

| # | Carte | Apport |
|---|---|---|
| 376 | L'ajout par un administrateur devient une commande du domaine | `add_member`, `DejaMembre`, l'événement |
| 377 | Le dépôt sait chercher dans l'annuaire de la plateforme | une requête, et son piège de jointure |
| 378 | Ajouter un membre, côté applicatif | use case, courtoisie par email |
| 379 | `auth` sait créer un compte sans mot de passe | use case, l'email de définition |
| 380 | Le widget de création de compte, fourni par `auth` | fragment, gabarit, feuille, `HX-Trigger` |
| 381 | L'onglet Ajout direct : le cadre, la recherche, les candidats | widget de lecture, trois états |
| 382 | Ajouter un coach déjà inscrit | action, re-rendu, journal de session |
| 383 | Créer un compte et ajouter | injection du widget, câblage de `accountCreated` |
| 384 | L'onglet Ajout direct sous Playwright | couverture e2e |

## Ce qui commande l'ordre

**376 dépend de 365 et 375** — le premier donne à l'agrégat sa forme et ses
premières méthodes, le second répare son chargement. Sans 375, `add_member`
vérifie « est-il déjà membre ? » sur une liste amputée.

**377 est parallélisable** avec 376 : c'est du SQL, il ne touche pas au domaine.

**378 après 376 et 377.**

**379 et 380 sont dans `auth`, et indépendantes de tout le reste.** Elles
peuvent se faire en premier, en dernier, ou pendant. 380 dépend de 379.

**380 est une carte à part**, bien qu'elle n'ait aucune valeur tant que `spaces`
ne l'affiche pas. Elle vit entièrement dans `auth` — use case, gabarit, feuille,
tests — et un BC dont on maintient l'indépendance ne se livre pas dans le même
commit que son consommateur. Son affichage est le sujet de la 383.

**381 après 377**, qui lui donne sa lecture.

**382 après 378 et 381.** **383 après 380 et 382** — elle réutilise l'action
d'ajout de la 382 et le widget de la 380.

**384 en dernier.**

```
375 ──► 365 ──► 376 ──┐
                      ├──► 378 ──┐
        377 ──────────┴──► 381 ──┴──► 382 ──┐
                                            ├──► 384
        379 ──► 380 ────────────────────────┴──► 383
```

## Ce que les cartes ne redisent pas

La conception vit dans les fichiers de spec de ce dossier. Chaque carte y
renvoie plutôt que de la recopier.

## Le contrat que rien ne vérifie

`accountCreated` — son nom et ses clés `coach_id` et `name` — franchit la
frontière entre `auth` et `spaces` **par le navigateur**. Ni le compilateur, ni
`cargo test`, ni `check-arch` ne le voient.

Deux cartes le portent, 380 qui l'émet et 383 qui l'écoute, et **384 est son
seul filet**. C'est écrit dans les trois.

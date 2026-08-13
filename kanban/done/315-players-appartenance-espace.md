# BC `players` — le joueur doit appartenir à l'espace du chemin

**Priorité : haute**
**Bloque :** `309-players-customisation-e2e.md`
**Contexte :** `players` — autorisation

## Le défaut

`can_customise` et `can_spend_spp` demandent tous deux :

```rust
find_member_profile(user_id, space_id)   // space_id vient du CHEMIN
```

Ils vérifient que l'appelant est admin **de l'espace nommé dans l'URL**. Jamais
que le joueur visé appartient à cet espace. Le `space_id` du chemin est
contrôlé par l'appelant.

**Être admin d'un espace quelconque suffit donc à agir sur n'importe quel
joueur de l'application**, en écrivant son propre espace dans l'URL.

La branche « admin de compétition » est saine dans les deux fonctions : elle
part de l'équipe du joueur, donc d'une donnée que l'appelant ne choisit pas.
C'est uniquement la branche « admin d'espace » qui fait confiance à l'URL.

## Constaté, pas supposé

Sur le serveur de développement, joueur appartenant à un espace tiers :

```
POST /app/<Espace E2E>/players/<joueur d'un autre espace>/customisation/spp/add
→ 200, ligne écrite : [{"Spp": {"id": "l1", "amount": 5}}]
```

Le panneau de customisation répond « Mode customisation » au lieu de retomber
sur le journal, et le panier créé est estampillé de l'espace **du chemin**, pas
de celui du joueur.

## Pourquoi c'est urgent maintenant

Le défaut **préexiste** : `can_customise` gardait déjà l'affichage du bouton.
Mais les cartes 307 et 308 viennent d'en faire la garde de **sept endpoints
d'écriture** touchant compétences, caractéristiques, prix et SPP. Ce qui n'était
qu'un affichage indu est devenu une porte.

## Étendue mesurée

**Aucun des quinze fichiers de `players/io/web/` ne vérifie l'appartenance** —
contrôleurs et widgets confondus, zéro occurrence.

| Fonction | Endpoints couverts | Atténuation |
|---|---|---|
| `can_customise` | le panneau + les 7 `POST` de customisation | aucune |
| `can_spend_spp` | achat de compétence, augmentation de caractéristique | ses appelants exigent aussi `in_player_improvement_phase` — la fenêtre est plus étroite, le défaut identique |

## Le correctif

`Player` porte son `space_id` (`domain/player.rs`). Il suffit de le comparer à
celui du chemin.

**`404`, et non `403`.** Un `403` confirmerait l'existence d'un joueur d'un
autre espace — c'est précisément ce qu'un appelant cherchant à énumérer
voudrait apprendre. Pour lui, un joueur hors de son espace n'existe pas.

Le contrôle va **avant** l'autorisation, pas après : il ne s'agit pas de savoir
qui a le droit, mais de savoir de quoi on parle.

---

## Checklist

- [x] Comparaison espace du joueur / espace du chemin, en amont des deux
      fonctions d'autorisation — `space_scope::charger_joueur_de_l_espace`
- [x] `404` sur divergence, sur **tous** les endpoints concernés
- [x] Le panier créé porte l'espace du joueur — les deux ne peuvent plus diverger
- [ ] Test : un admin d'espace A ne peut ni voir ni modifier un joueur de B — **reporté en 309**, scénario 11
- [ ] Test : le cas nominal — même espace — reste inchangé — **reporté en 309**
- [x] Vérifier `player_debug_controller` et les widgets — **tous étaient vulnérables**, voir ci-dessous

## Ce que cette carte ne fait pas

Elle ne traite que `players`. Sept autres BCs exposent des routes
`/app/{space_id}/…` et n'ont pas été audités — c'est l'objet de la carte 316.

## Réalisé

`space_scope::charger_joueur_de_l_espace` est désormais le **seul** moyen
d'obtenir un `Player` depuis la couche web de ce BC. Vérifié : plus aucun
`find_by_id(&PlayerId(...))` n'y subsiste ailleurs.

Six sites de chargement y sont passés, tous vulnérables :

| Site | Ce qu'il exposait |
|---|---|
| `customisation_access` | le panneau **et les sept `POST`** |
| `purchase_skill_controller` | achat de compétence |
| `increase_stat_controller` | augmentation de caractéristique |
| `player_detail_controller` | la fiche entière |
| `evolution_journal_widget` | le journal |
| `spp_spending_widget` | le panneau de dépense |

**`player_debug_controller` était le pire** : il recevait le `space_id` sous le
nom `_space_id`, c'est-à-dire explicitement ignoré, et servait l'intégralité de
l'état d'un joueur — événements compris — depuis n'importe quel espace.

## Vérifié sur le serveur

Joueur appartenant à un espace tiers, appelé depuis un espace dont l'appelant
est admin :

```
widgets/customisation      404      (était : « Mode customisation »)
detail                     404
debug                      404
widgets/evolution-journal  404
POST customisation/spp/add 404      (était : 200, ligne écrite)
```

Et le cas nominal, même joueur depuis son espace réel : `200`.

## Les tests partent en 309

Le contrôle porte sur un `AppState` : il n'est pas testable unitairement, faute
du harnais décrit par la carte 311. La vérification est donc **empirique
ci-dessus**, et un scénario 11 est ajouté à la carte 309 pour la figer.

C'est le deuxième report du même genre après la carte 308. Le manque coûte
maintenant deux fois.

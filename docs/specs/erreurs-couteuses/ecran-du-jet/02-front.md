# L'écran du jet · Phase 2 : architecture front

**Maquette** : `assets/rawpages/html/app-team-costly-mistakes.html`

## Aucun widget

Le `CLAUDE.md` réserve le pattern « page d'assemblage à widgets » aux pages de
trois sections interactives et plus, ou combinant plusieurs BCs, et dit
explicitement quand **ne pas** l'appliquer : « page simple avec un formulaire et
une réponse ».

C'est exactement cet écran. Une table statique, un bouton, une réponse. Tout
vient de `teams`, rien d'un autre BC. Le découper en widgets créerait des
endpoints et des événements DOM pour coordonner ce qui tient dans une page.

| Élément | Nature | Chargement |
|---|---|---|
| En-tête d'équipe | statique | avec la page |
| Table de déclenchement | statique, tranche du coach mise en évidence | avec la page |
| Zone de jet | bouton + dé | avec la page |
| Résultat | **fragment**, en réponse au POST | `hx-post`, swap sur la zone |
| Bouton de sortie | navigation | avec la page, activé après le jet |

## Le seul échange serveur

```
POST  /app/{space_id}/teams/{team_id}/costly-mistakes/roll
  →   fragment HTML : le verdict, le calcul, la case touchée dans la table
```

Pas de payload : le serveur n'a besoin de rien d'autre que l'identité de
l'équipe. **Le client n'envoie aucun jet** — il ne fait que demander qu'on en
tire un. C'est ce qui rend le tirage inviolable.

La réponse remplace la zone de jet et de résultat d'un seul `hx-swap`, et
réaffiche la table avec sa case touchée : trois morceaux, un seul échange, pas
d'état à recoller côté client.

## Ce qui reste front

**L'animation du dé, et rien d'autre.** Elle démarre au clic, tourne pendant la
requête, et s'arrête sur la valeur rendue par le serveur.

**Il lui faut une durée plancher** — un peu plus d'une seconde. Sans elle, une
réponse en 80 ms ferait clignoter le dé et le résultat surgirait sans que rien ne
se soit passé. Ce n'est pas de la décoration : le suspense est la seule chose que
le coach obtienne en échange de ne pas lancer le dé lui-même.

Techniquement, la réponse arrive avant la fin de l'animation. Le fragment est
donc reçu puis affiché à l'échéance — `htmx.ajax` piloté par le composant Alpine
qui tient l'animation, plutôt qu'un `hx-post` nu qui échangerait aussitôt.

## Le double clic

Le bouton se désactive au premier clic et ne se réactive jamais. Mais **la garde
qui compte est ailleurs** : le domaine refuse un second jet parce que la phase
n'est plus `CostlyMistakes` — `CostlyMistakesApplied` repose `ReadyToPlay`. Le
front se contente de ne pas provoquer une erreur que le serveur sait déjà
refuser.

## Une seule entrée : le bandeau de la fiche d'équipe

L'écran s'atteint comme les autres phases, par le bandeau d'état de la fiche
d'équipe — `BannerCtaVm::Navigate`, le type existe déjà et sert au recrutement
(`team_detail.rs:162`) comme aux renvois :

```rust
(Enrolled, Some(CostlyMistakes)) => Some(Self {
    css_variant: "phase".into(),
    icon: "💸".into(),
    title: "Erreurs coûteuses.".into(),
    detail: "Un jet décide de ce qu'il reste de votre trésorerie.".into(),
    ctas: vec![BannerCtaVm::Navigate {
        label: "Lancer le dé →".into(),
        href: app_routes.teams.costly_mistakes_page(space_id, &team_id),
    }],
}),
```

**La page n'existe que pendant la phase.** Hors `CostlyMistakes`, elle redirige
vers la fiche d'équipe — c'est déjà ce que font le recrutement et les renvois,
dont le commentaire dit qu'une page « n'a plus lieu d'être une fois la phase
close » (`validate_phase_actions.rs:129`).

### Ce qu'on abandonne, et ce que ça coûte

La consultation du dernier jet est **écartée**. Elle demandait un champ dérivé
sur l'agrégat, un second rendu du contrôleur, un CTA conditionnel dans le
bandeau, et une règle de durée — pour une page qu'on regarde une fois.

**Le prix est connu et accepté** : un coach qui recharge sa page après le jet ne
reverra pas son résultat. Le fragment est dans son navigateur, pas ailleurs, et
la phase est déjà passée à `ReadyToPlay`.

Le montant, lui, n'est pas perdu : le mouvement figure au grand livre avec le
motif `CostlyMistake`, et deviendra lisible le jour où l'onglet Trésorerie
existera (carte 48). C'est le bon endroit pour cette question — on va y chercher
où est passé l'argent, pas revoir un dé.

## Qui peut lancer le dé

Le **coach propriétaire**, un **administrateur d'espace**, un **administrateur de
la compétition** — la règle exacte de la carte 389, qui introduit
`ITeamAccessPort` dans `teams`.

**À ne pas dupliquer** : si la 389 est livrée avant, le port existe et cette
fonctionnalité s'en sert ; sinon, celle qui arrive la première le crée. Le jet a
un effet financier, et l'écran est atteignable par quiconque connaît l'URL.

## Ce que la page ne fait pas

- **Aucune redirection automatique** après le jet. Le coach lit son résultat et
  sort quand il veut, par le bouton « L'équipe est prête à jouer ».
- **Aucun rappel du panier de recrutement ni des renvois** : ces phases sont
  closes, leurs écrans n'ont plus lieu d'être.

## Règles métier tranchées

| Question | Décision |
|---|---|
| Qui lance le dé ? | propriétaire, admin d'espace, admin de compétition — règle de la carte 389 |
| Comment atteint-on la page ? | par le bandeau d'état, comme les autres phases |
| Que voit un visiteur hors phase ? | rien : redirection vers la fiche d'équipe |
| Combien de temps le résultat est-il consultable ? | le temps de la page. Pas de consultation ultérieure — écartée délibérément |

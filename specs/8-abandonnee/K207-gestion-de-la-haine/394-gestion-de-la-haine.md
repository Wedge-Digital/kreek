> **Carte annulée le 2026-08-26 — ses six questions ont trouvé leur réponse.**
>
> Elle était un marque-place : « la Haine doit être gérée, rien n'en existe ».
> La fonctionnalité est désormais spécifiée en entier — `docs/specs/haine/`,
> huit phases — et découpée en sept cartes, **399 à 405**.
>
> Les réponses, dans l'ordre où cette carte posait les questions :
>
> - **de quoi il s'agit** — un trait gagné par un joueur blessé, gratuit : ni SPP
>   payé, ni valeur ajoutée ;
> - **ce qu'elle relie** — un joueur et un mot-clef parmi ceux que le corpus
>   déclare haïssables, trente sur trente-huit ;
> - **comment elle naît** — sur trois blessures seulement, Amoché, Blessure
>   Sérieuse et Séquelle, déclarée par le coach à la saisie de l'action ;
> - **son effet** — aucun à ce stade : les mots-clefs des lignes de roster
>   rendront la comparaison possible, mais aucune règle n'est spécifiée ;
> - **qui la saisit** — le coach, dans le panneau d'action du rapport de match ;
> - **le BC** — `match_report` pour la saisie, `players` pour le trait acquis,
>   `references` pour le corpus.
>
> Une question qu'elle ne posait pas et qui a compté : **le lien vers la
> compétence est déclaré par le corpus**, jamais déduit d'une convention de
> nommage, et figé dans l'action au moment du choix.

# Gestion de la Haine

**Priorité : à définir**
**Statut : marque-place** — créée pour ne pas être oubliée, pas encore raffinée.

## Ce qu'on sait

La Haine doit être gérée par la plateforme. Rien n'en existe aujourd'hui :
aucune trace du terme dans le code, dans le corpus de référence, ni dans les
autres cartes.

## Ce qu'il faudra préciser pour la raffiner

- De quoi il s'agit : règle du règlement officiel, ou règle maison de la ligue ?
- Ce que la Haine relie : deux équipes, une équipe et un roster, un joueur et un
  adversaire ?
- Comment elle naît, comment elle s'éteint, et si elle survit à la saison.
- Son effet : sur un match (jets, appariements), sur le classement, ou seulement
  sur l'affichage.
- Qui la saisit : le coach, l'organisateur, ou la plateforme d'elle-même à
  partir des rapports de match.
- Le BC concerné : `match_report`, `competitions`, `teams` — ou un nouveau.

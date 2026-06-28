# Match Report — Spec index

## Pages

| Page | Front | Back | DTOs | Use cases | Domaine | Intégration | Cartes |
|---|---|---|---|---|---|---|---|
| step1-selection | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| step2-avant-match | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| step2-inducements | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| step3-4-actions | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| step5-apres-match | | | | | | | |
| recap | | | | | | | |

## Règles métier transverses (identifiées en phase 1)

### Accès et permissions

- **Coach lambda** : ne peut créer/éditer un rapport que pour un match impliquant une de ses équipes enrolled
- **Admin compétition** : choix libre des deux équipes parmi les enrolled de la saison
- **Admin espace** : même chose que l'admin compétition
- Un rapport soumis n'est modifiable que par un admin (espace ou compétition)

### Deux modes de création

- **Pairing pré-programmé** : le BC competitions émet un app event à la création du pairing → le BC match_report crée automatiquement un match report en phase 1 (pré-rempli). L'utilisateur arrive en édition via `/match-report/{match_report_id}`.
- **Formulaire vierge** : le coach ou admin sélectionne manuellement compétition/saison/journée/équipes via `/match-report/new`. Utilisé quand la compétition ne programme pas les pairings à l'avance.

### Règles de sélection des équipes

- Les poules ne filtrent PAS les équipes sélectionnables — toute équipe enrolled dans la saison peut affronter toute autre
- Seules les équipes Enrolled + `ReadyToPlay` sont sélectionnables dans les selects
- Plusieurs rapports en phase Draft peuvent coexister pour la même équipe (cas du calendrier pré-programmé)
- Le verrouillage s'applique au `SelectionConfirmed` : les deux équipes passent en `MatchReporting` (nouveau variant `GamePhase` dans le BC teams) via un app event `MatchReportConfirmed`
- Si une équipe n'est plus en `ReadyToPlay` au moment de la confirmation, celle-ci est refusée
- Les deux équipes doivent être différentes

### Règles de match

- Journaliers ajoutés automatiquement si < 11 joueurs
- Inducements : équipe forte dépense sa trésorerie, équipe faible dépense (différence TV + dépenses adverses + 50 kPo trésorerie)
- Inducements filtrés par roster + règles compétition
- Score déduit des TDs (pas de saisie manuelle)
- Fan factor : dedicated fans + D3
- Gains : (somme fan factors / 2) × 10 000 + nb TD × 10 000 pO
- MVP : pas de limite par équipe, warning si aucun MVP désigné
- Actions par tour (1-16) : TD, Passe, Interception, Agression, Lancer, Sortie, Blessé, MVP
- Blessures : Commotion (1-8) | Amoché (9-10) | Blessure Sérieuse/Niggling (11-12) | Séquelle + stat (13-14) | Mort (15-16)
- Niggling (11-12) : affect uniquement les joueurs réguliers, pas les temporaires
- Joueurs temporaires (star, merc, journalier) identifiés par TempPlayerId (UUID match-scoped), pas de nom sauf star players
- Journaliers : max(0, 11 - joueurs_disponibles), info fournie par BC Players
- Mercenaires : déclarés comme inducements (même flux que star players en step 2)
- Broadcast vers BC Players : tous les événements d'action → BC Players calcule les SPP

/* Coquille mobile des maquettes « parcours mobile » : l'en-tête et la barre
   d'onglets, posés une fois ici plutôt que recopiés dans chaque écran.

   Le balisage est celui de `src/web/templates/app-menu.html` (`.mobile-header`,
   `.mobile-tabbar`, `.mobile-tab`) : le style vient donc du vrai
   `layout-app.css`. Seuls changent les onglets — la proposition — et la
   pastille, nouvelle (`app-mobile.css`).

   L'onglet actif se lit sur `<body data-tab="…">`. */
(function () {
  var TABS = [
    ['home', '🏠', 'Accueil', 2],
    ['teams', '🔍', 'Équipes', 0],
    ['matches', '🏈', 'Matchs', 1],
    ['league', '🏆', 'Ligue', 0],
    ['me', '👤', 'Moi', 0],
  ];

  function header() {
    return '<div class="mobile-header">'
      + '<div class="mobile-header-space">'
      + '<div class="mobile-header-space-logo">TC</div>'
      + '<span class="mobile-header-space-name">Tribu Celtique</span>'
      + '<span class="mobile-header-space-chevron">▼</span>'
      + '</div>'
      + '<div class="mobile-header-actions">'
      + '<a class="mobile-header-btn mobile-header-btn--notif">🔔<span class="mobile-header-dot"></span></a>'
      + '</div>'
      + '</div>';
  }

  function tabbar(active) {
    return '<div class="mobile-tabbar">' + TABS.map(function (t) {
      var badge = t[3] ? '<span class="mobile-tab-badge">' + t[3] + '</span>' : '';
      return '<div class="mobile-tab' + (t[0] === active ? ' active' : '') + '">'
        + '<span class="mobile-tab-icon">' + t[1] + '</span>'
        + '<span class="mobile-tab-label">' + t[2] + '</span>' + badge
        + '</div>';
    }).join('') + '</div>';
  }

  document.addEventListener('DOMContentLoaded', function () {
    var zone = document.querySelector('.menu-zone');
    if (zone) zone.innerHTML = header() + tabbar(document.body.dataset.tab);
  });
})();

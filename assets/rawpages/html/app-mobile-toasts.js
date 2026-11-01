/* Prototype des notifications poussées (SSE) : l'arrivée des toasts.

   Le balisage et les classes sont ceux du toast réel du layout
   (`components/toast.css` : `.toast-zone`, `.toast`, `.toast--enter`) ; seul
   le placement change sur mobile (`app-mobile.css`, `.mb-toast-zone`).

   Un écran l'active par `<body data-toasts>`. Les deux messages passent l'un
   après l'autre, en boucle, pour qu'on voie leur entrée et leur sortie. */
(function () {
  var MESSAGES = [
    ['🏈 Saisie du match démarrée par Morgane', 'Nains de Fer – London Rats · Ligue de Condate J4'],
    ['🏈 Touchdown du 9 des London Rats saisi par Morgane', 'Nains de Fer 2 – 1 London Rats · Voir le match ›'],
  ];
  var SHOWN = 3500, GAP = 1200;

  function toast(zone, message) {
    var el = document.createElement('div');
    el.className = 'toast mb-toast toast--enter';
    el.innerHTML = message[0] + '<span class="mb-toast-sub">' + message[1] + '</span>';
    zone.appendChild(el);
    requestAnimationFrame(function () {
      requestAnimationFrame(function () { el.classList.remove('toast--enter'); });
    });
    setTimeout(function () {
      el.classList.add('toast--enter');
      setTimeout(function () { el.remove(); }, 250);
    }, SHOWN);
  }

  document.addEventListener('DOMContentLoaded', function () {
    if (!('toasts' in document.body.dataset)) return;
    var zone = document.createElement('div');
    zone.className = 'toast-zone mb-toast-zone';
    document.body.appendChild(zone);
    var i = 0;
    (function next() {
      toast(zone, MESSAGES[i % MESSAGES.length]);
      i += 1;
      setTimeout(next, SHOWN + GAP);
    })();
  });
})();

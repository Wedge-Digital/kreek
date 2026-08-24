/**
 * <kreek-select> — Web Component select searchable avec chargement JSON.
 *
 * Attributs :
 *   name           — nom du champ form (hidden input)
 *   url            — URL GET retournant un JSON array
 *   value-field    — champ JSON pour la valeur (défaut: "id")
 *   label-field    — champ JSON pour le texte affiché (défaut: "name")
 *   search-fields  — champs JSON sur lesquels la recherche s'applique, séparés par virgule (défaut: label-field)
 *   placeholder    — texte placeholder (défaut: "Sélectionnez…")
 *   event          — nom de l'événement DOM émis sur document.body au changement
 *   event-fields   — champs JSON inclus dans le payload de l'événement, séparés par virgule
 *   selected-value — valeur pré-sélectionnée
 *   disabled       — désactive le composant
 *   option-template — ID d'un <template> pour le rendu riche des options
 *   selected-template — ID d'un <template> pour le rendu de l'élément sélectionné
 *   multiple       — active la sélection multiple (badges)
 *   listen         — nom d'un événement DOM (sur body) qui déclenche un rechargement des données
 *   listen-param   — champ du detail de l'événement dont la valeur est utilisée
 *   listen-query   — nom du query param envoyé à l'URL (défaut: même valeur que listen-param)
 *   auto-select-first — sélectionne automatiquement le premier item après un fetch cascade
 *
 * Options statiques — alternative à `url`, pour une liste figée qui ne justifie
 * pas un endpoint JSON. Se déclare en enfants `<option>`, comme un select natif :
 *
 *   <kreek-select name="onglet">
 *     <option value="standings" selected>Classement</option>
 *     <option value="resultats">Résultats</option>
 *   </kreek-select>
 *
 * `value` est facultatif (le texte fait alors office de valeur) et `selected`
 * équivaut à l'attribut `selected-value`. Si des `<option>` sont présentes,
 * `url` est ignorée.
 *
 * Les templates utilisent %field% comme placeholders, remplacés par les valeurs JSON.
 */
(function () {
  class KreekSelect extends HTMLElement {
    connectedCallback() {
      this._url = this.getAttribute('url') || '';
      this._valueField = this.getAttribute('value-field') || 'id';
      this._labelField = this.getAttribute('label-field') || 'name';
      this._searchFields = (this.getAttribute('search-fields') || '')
        .split(',').map(function (s) { return s.trim(); }).filter(Boolean);
      this._placeholder = this.getAttribute('placeholder') || 'Sélectionnez…';
      this._eventName = this.getAttribute('event') || '';
      this._eventFields = (this.getAttribute('event-fields') || '')
        .split(',').map(function (s) { return s.trim(); }).filter(Boolean);
      this._fieldName = this.getAttribute('name') || '';
      this._selectedValue = this.getAttribute('selected-value') || '';
      this._disabled = this.hasAttribute('disabled');
      this._multiple = this.hasAttribute('multiple');
      this._optionTemplateId = this.getAttribute('option-template') || '';
      this._selectedTemplateId = this.getAttribute('selected-template') || '';
      this._items = [];
      this._open = false;
      this._filterText = '';
      this._selectedItem = null;
      this._selectedItems = [];
      this._highlightIdx = -1;

      this._listenEvent = this.getAttribute('listen') || '';
      this._listenParam = this.getAttribute('listen-param') || '';
      this._listenQuery = this.getAttribute('listen-query') || this._listenParam;
      this._autoSelectFirst = this.hasAttribute('auto-select-first');

      var self = this;
      var start = function () {
        // Lues **avant** `_render()`, qui vide le contenu de l'élément.
        var inlineItems = self._readInlineOptions();

        self._render();
        if (inlineItems.length) {
          self._items = inlineItems;
          self._renderOptions();
          if (self._selectedValue) self._preselectValue(self._selectedValue);
        } else if (self._url && !self._listenEvent) {
          self._fetchData();
        }
        self._setupListener();
      };

      // Pendant l'analyse initiale du document, `connectedCallback` se déclenche
      // dès la balise ouvrante : les `<option>` enfants n'existent pas encore, et
      // les lire donnerait une liste vide. On attend alors la fin de l'analyse.
      // Sur un fragment injecté par HTMX, les enfants sont déjà là — et un select
      // alimenté par `url` n'a de toute façon rien à attendre.
      if (document.readyState === 'loading' && !this._url && !this.querySelector('option')) {
        document.addEventListener('DOMContentLoaded', start, { once: true });
      } else {
        start();
      }
    }

    /**
     * Options statiques déclarées en enfants `<option>` — voir l'en-tête du
     * fichier. Retourne un tableau vide s'il n'y en a pas, auquel cas le
     * composant retombe sur son chargement par `url`.
     */
    _readInlineOptions() {
      var self = this;
      return Array.prototype.map.call(this.querySelectorAll('option'), function (opt) {
        var label = opt.textContent.trim();
        var value = opt.hasAttribute('value') ? opt.getAttribute('value') : label;
        if (opt.hasAttribute('selected') && !self._selectedValue) self._selectedValue = value;
        var item = {};
        item[self._valueField] = value;
        item[self._labelField] = label;
        return item;
      });
    }

    disconnectedCallback() {
      this._removeOutsideListener();
      this._removeListener();
    }

    _setupListener() {
      if (!this._listenEvent) return;
      var self = this;
      this._listenHandler = function (e) {
        var paramValue = self._listenParam && e.detail ? e.detail[self._listenParam] : '';
        self._reset();
        if (!paramValue) return;
        self._autoSelectOnNextFetch = self._autoSelectFirst;
        var sep = self._url.indexOf('?') === -1 ? '?' : '&';
        var fetchUrl = self._url + sep + encodeURIComponent(self._listenQuery) + '=' + encodeURIComponent(paramValue);
        self._fetchUrl(fetchUrl);
      };
      document.body.addEventListener(this._listenEvent, this._listenHandler);
    }

    _removeListener() {
      if (this._listenHandler && this._listenEvent) {
        document.body.removeEventListener(this._listenEvent, this._listenHandler);
        this._listenHandler = null;
      }
    }

    _reset() {
      this._items = [];
      this._selectedItem = null;
      this._selectedItems = [];
      this._hidden.value = '';
      if (this._multiple) {
        this._renderBadges();
      } else {
        this._display.className = 'ks-display ks-placeholder';
        this._display.innerHTML = '';
        this._display.textContent = this._placeholder;
      }
      this._renderOptions();
    }

    _render() {
      this.innerHTML = '';
      this.classList.add('kreek-select');
      if (this._multiple) this.classList.add('ks-multiple');

      this._hidden = document.createElement('input');
      this._hidden.type = 'hidden';
      this._hidden.name = this._fieldName;
      this._hidden.value = this._selectedValue;
      this.appendChild(this._hidden);

      this._control = document.createElement('div');
      this._control.className = 'ks-control';
      if (this._disabled) this._control.classList.add('ks-disabled');
      this.appendChild(this._control);

      if (this._multiple) {
        this._badgesContainer = document.createElement('div');
        this._badgesContainer.className = 'ks-badges';
        this._control.appendChild(this._badgesContainer);
        this._placeholderEl = document.createElement('span');
        this._placeholderEl.className = 'ks-display ks-placeholder';
        this._placeholderEl.textContent = this._placeholder;
        this._badgesContainer.appendChild(this._placeholderEl);
      } else {
        this._display = document.createElement('span');
        this._display.className = 'ks-display ks-placeholder';
        this._display.textContent = this._placeholder;
        this._control.appendChild(this._display);
      }

      this._arrow = document.createElement('span');
      this._arrow.className = 'ks-arrow';
      this._arrow.innerHTML = '&#9662;';
      this._control.appendChild(this._arrow);

      this._dropdown = document.createElement('div');
      this._dropdown.className = 'ks-dropdown';
      this._dropdown.style.display = 'none';
      this.appendChild(this._dropdown);

      this._searchInput = document.createElement('input');
      this._searchInput.type = 'text';
      this._searchInput.className = 'ks-search';
      this._searchInput.placeholder = 'Rechercher…';
      this._dropdown.appendChild(this._searchInput);

      this._optionsList = document.createElement('div');
      this._optionsList.className = 'ks-options';
      this._dropdown.appendChild(this._optionsList);

      var self = this;
      this._control.addEventListener('click', function () {
        if (!self._disabled) self._toggle();
      });
      this._searchInput.addEventListener('input', function () {
        self._filterText = self._searchInput.value.toLowerCase();
        self._renderOptions();
      });
      this._searchInput.addEventListener('keydown', function (e) {
        self._handleKeydown(e);
      });
    }

    _fetchData() {
      this._fetchUrl(this._url);
    }

    _fetchUrl(url) {
      var self = this;
      fetch(url)
        .then(function (r) { return r.json(); })
        .then(function (data) {
          self._items = Array.isArray(data) ? data : [];
          self._renderOptions();
          if (self._selectedValue) {
            self._preselectValue(self._selectedValue);
          } else if (self._autoSelectOnNextFetch && self._items.length > 0) {
            self._selectItem(self._items[0], true);
          }
          self._autoSelectOnNextFetch = false;
        })
        .catch(function (err) {
          console.error('kreek-select fetch error:', err);
          self._items = [];
          self._renderOptions();
        });
    }

    _renderTemplate(templateId, item) {
      var tpl = document.getElementById(templateId);
      if (!tpl) return null;
      var html = tpl.innerHTML;
      var keys = Object.keys(item);
      for (var i = 0; i < keys.length; i++) {
        html = html.split('%' + keys[i] + '%').join(item[keys[i]] != null ? item[keys[i]] : '');
      }
      var frag = document.createElement('div');
      frag.innerHTML = html.trim();
      return frag;
    }

    _preselectValue(val) {
      var vf = this._valueField;
      for (var i = 0; i < this._items.length; i++) {
        if (String(this._items[i][vf]) === String(val)) {
          this._selectItem(this._items[i], true);
          this._selectedValue = '';
          return;
        }
      }
    }

    _renderOptions() {
      var self = this;
      var vf = this._valueField;
      var lf = this._labelField;
      var filter = this._filterText;
      this._optionsList.innerHTML = '';
      this._highlightIdx = -1;

      var fields = self._searchFields.length > 0 ? self._searchFields : [lf];
      var filtered = this._items.filter(function (item) {
        if (!filter) return true;
        for (var i = 0; i < fields.length; i++) {
          if (String(item[fields[i]] || '').toLowerCase().indexOf(filter) !== -1) return true;
        }
        return false;
      });

      if (filtered.length === 0) {
        var empty = document.createElement('div');
        empty.className = 'ks-option ks-empty';
        empty.textContent = 'Aucun résultat';
        this._optionsList.appendChild(empty);
        return;
      }

      this._filteredItems = filtered;
      for (var i = 0; i < filtered.length; i++) {
        (function (idx) {
          var item = filtered[idx];
          var el = document.createElement('div');
          el.className = 'ks-option';
          var isSelected = self._multiple
            ? self._selectedItems.some(function (s) { return String(s[vf]) === String(item[vf]); })
            : (self._selectedItem && String(item[vf]) === String(self._selectedItem[vf]));
          if (isSelected) el.classList.add('ks-selected');
          if (self._optionTemplateId) {
            var rendered = self._renderTemplate(self._optionTemplateId, item);
            if (rendered) el.appendChild(rendered);
            else el.textContent = item[lf];
          } else {
            el.textContent = item[lf];
          }
          el.addEventListener('click', function (e) {
            e.stopPropagation();
            if (self._multiple) {
              self._toggleItem(item);
            } else {
              self._selectItem(item, false);
              self._close();
            }
          });
          self._optionsList.appendChild(el);
        })(i);
      }
    }

    _toggleItem(item) {
      var vf = this._valueField;
      var idx = -1;
      for (var i = 0; i < this._selectedItems.length; i++) {
        if (String(this._selectedItems[i][vf]) === String(item[vf])) { idx = i; break; }
      }
      if (idx >= 0) {
        this._selectedItems.splice(idx, 1);
      } else {
        this._selectedItems.push(item);
      }
      this._syncMultiValue();
      this._renderBadges();
      this._renderOptions();
      this._emitMultiEvent();
    }

    _removeItem(item) {
      var vf = this._valueField;
      this._selectedItems = this._selectedItems.filter(function (s) {
        return String(s[vf]) !== String(item[vf]);
      });
      this._syncMultiValue();
      this._renderBadges();
      this._renderOptions();
      this._emitMultiEvent();
    }

    _syncMultiValue() {
      var vf = this._valueField;
      var vals = this._selectedItems.map(function (s) { return s[vf]; });
      this._hidden.value = JSON.stringify(vals);
    }

    _renderBadges() {
      var self = this;
      var lf = this._labelField;
      this._badgesContainer.innerHTML = '';
      if (this._selectedItems.length === 0) {
        var ph = document.createElement('span');
        ph.className = 'ks-display ks-placeholder';
        ph.textContent = this._placeholder;
        this._badgesContainer.appendChild(ph);
        return;
      }
      for (var i = 0; i < this._selectedItems.length; i++) {
        (function (item) {
          var badge = document.createElement('span');
          badge.className = 'ks-badge';
          badge.textContent = item[lf];
          var rm = document.createElement('span');
          rm.className = 'ks-badge-remove';
          rm.innerHTML = '&times;';
          rm.addEventListener('click', function (e) {
            e.stopPropagation();
            self._removeItem(item);
          });
          badge.appendChild(rm);
          self._badgesContainer.appendChild(badge);
        })(this._selectedItems[i]);
      }
    }

    _emitMultiEvent() {
      if (!this._eventName) return;
      var vf = this._valueField;
      var lf = this._labelField;
      var ef = this._eventFields;
      var values = this._selectedItems.map(function (item) {
        var o = {};
        o[vf] = item[vf];
        o[lf] = item[lf];
        for (var i = 0; i < ef.length; i++) {
          if (item[ef[i]] !== undefined) o[ef[i]] = item[ef[i]];
        }
        return o;
      });
      this._emit(this._eventName, { items: values });
    }

    _selectItem(item, isPreselect) {
      var vf = this._valueField;
      var lf = this._labelField;
      this._selectedItem = item;
      this._hidden.value = item[vf];
      if (this._selectedTemplateId) {
        var rendered = this._renderTemplate(this._selectedTemplateId, item);
        if (rendered) {
          this._display.textContent = '';
          this._display.innerHTML = '';
          this._display.appendChild(rendered);
        } else {
          this._display.textContent = item[lf];
        }
      } else {
        this._display.textContent = item[lf];
      }
      this._display.classList.remove('ks-placeholder');
      this._renderOptions();

      if (this._eventName) {
        var payload = {};
        payload[vf] = item[vf];
        payload[lf] = item[lf];
        var ef = this._eventFields;
        for (var i = 0; i < ef.length; i++) {
          if (item[ef[i]] !== undefined) payload[ef[i]] = item[ef[i]];
        }
        this._emit(this._eventName, payload);
      }

      // Un `change` sur le composant lui-même, comme tout contrôle de
      // formulaire. Il n'était pas émis, et `hx-trigger="change"` posé sur un
      // `<kreek-select>` ne partait donc jamais — sans que rien ne le signale.
      //
      // Le piège est vicieux : l'affichage est mis à jour localement, si bien
      // qu'un test qui vérifie ce qu'on lit à l'écran passe alors même
      // qu'aucune requête n'est partie. C'est ce qui a laissé passer le
      // changement de rôle de l'administration d'espace.
      //
      // `_emit` vise `document.body` : ce `change`-ci vise le composant, et
      // remonte par bulle jusqu'à ceux qui l'écoutent — dont HTMX.
      if (!isPreselect) {
        this.dispatchEvent(new Event('change', { bubbles: true }));
      }
    }

    _toggle() {
      if (this._open) this._close(); else this._openDropdown();
    }

    _openDropdown() {
      this._open = true;
      this._filterText = '';
      this._searchInput.value = '';
      this._renderOptions();
      this._dropdown.style.display = '';
      this._control.classList.add('ks-open');
      this._searchInput.focus();
      this._addOutsideListener();
    }

    _close() {
      this._open = false;
      this._dropdown.style.display = 'none';
      this._control.classList.remove('ks-open');
      this._removeOutsideListener();
    }

    _handleKeydown(e) {
      var items = this._filteredItems || [];
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        this._highlightIdx = Math.min(this._highlightIdx + 1, items.length - 1);
        this._updateHighlight();
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        this._highlightIdx = Math.max(this._highlightIdx - 1, 0);
        this._updateHighlight();
      } else if (e.key === 'Enter') {
        e.preventDefault();
        if (this._highlightIdx >= 0 && items[this._highlightIdx]) {
          if (this._multiple) {
            this._toggleItem(items[this._highlightIdx]);
          } else {
            this._selectItem(items[this._highlightIdx], false);
            this._close();
          }
        }
      } else if (e.key === 'Escape') {
        this._close();
      }
    }

    _updateHighlight() {
      var opts = this._optionsList.querySelectorAll('.ks-option:not(.ks-empty)');
      for (var i = 0; i < opts.length; i++) {
        opts[i].classList.toggle('ks-highlight', i === this._highlightIdx);
      }
      if (opts[this._highlightIdx]) {
        opts[this._highlightIdx].scrollIntoView({ block: 'nearest' });
      }
    }

    _emit(eventName, detail) {
      document.body.dispatchEvent(new CustomEvent(eventName, { detail: detail, bubbles: true }));
      var kebab = eventName.replace(/([a-z])([A-Z])/g, '$1-$2').toLowerCase();
      if (kebab !== eventName) {
        document.body.dispatchEvent(new CustomEvent(kebab, { detail: detail, bubbles: true }));
      }
      window.dispatchEvent(new CustomEvent(kebab, { detail: detail }));
    }

    _addOutsideListener() {
      var self = this;
      this._outsideHandler = function (e) {
        if (!self.contains(e.target)) self._close();
      };
      document.addEventListener('click', this._outsideHandler, true);
    }

    _removeOutsideListener() {
      if (this._outsideHandler) {
        document.removeEventListener('click', this._outsideHandler, true);
        this._outsideHandler = null;
      }
    }
  }

  customElements.define('kreek-select', KreekSelect);
})();

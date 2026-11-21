import {
  applyTheme,
  element,
  getOption,
  heroMark,
  heroOrnament,
  helperClient,
  icon,
  initTheme,
  keys,
  languages,
  loadSchemes,
  openFolder,
  openLauncher,
  setOption,
  setupPopup,
  shapeSvg,
  store,
  t,
  toast,
  variants,
} from './common.js';
import { customizerDefaults } from './src/core/customizer.js';

initTheme();

const panel = document.getElementById('panel');
const pagesNode = document.getElementById('pages');
const title = document.getElementById('page-title');
const popup = setupPopup('settings', {
  panel,
  onOpen: () => {
    const request = store.get('stellaria:settings:page');
    if (request && Date.now() - request.at < 5000) {
      store.set('stellaria:settings:page', null);
      showPage(request.page);
    }
  },
});
document.getElementById('close').addEventListener('click', () => popup.close());
document.body.addEventListener('pointerdown', event => {
  if (event.target === document.body) popup.close();
});

const schemeNames = {
  stellaria: 'Stellaria',
  catppuccin: 'Catppuccin',
  darkgreen: 'Dark Green',
  dracula: 'Dracula',
  everblush: 'Everblush',
  everforest: 'Everforest',
  gruvbox: 'Gruvbox',
  nord: 'Nord',
  oldworld: 'Oldworld',
  onedark: 'One Dark',
  rosepine: 'Rosé Pine',
  shadotheme: 'Shadotheme',
  solarized: 'Solarized',
  tokyonight: 'Tokyo Night',
};

const capitalize = value => value.charAt(0).toUpperCase() + value.slice(1);
const schemeBundle = await loadSchemes();

function switchControl(checked, onChange) {
  const node = element('button', { class: `switch${checked ? ' checked' : ''}` });
  node.addEventListener('click', () => {
    const next = !node.classList.contains('checked');
    node.classList.toggle('checked', next);
    onChange(next);
  });
  return node;
}

function row({ symbol, label, description, control }) {
  return element('div', { class: 'row' }, [
    symbol ? icon(symbol) : null,
    element('div', { class: 'row-text' }, [
      element('span', { class: 'row-title', text: label }),
      description ? element('span', { class: 'row-subtitle', text: description }) : null,
    ]),
    control,
  ]);
}

function section(label, children) {
  return element('div', { class: 'section' }, [element('span', { class: 'section-title', text: label }), ...children]);
}

function pageHero({ symbol, shape = 'cookie9', heading, text, chips = [], actions = [], art = null, backdrop = null }) {
  return element('div', { class: 'hero page-hero' }, [
    backdrop,
    heroOrnament(shape === 'flower' ? 'softBurst' : 'flower'),
    art ?? heroMark(shape, symbol),
    element('div', { class: 'page-hero-text' }, [
      element('h2', { text: heading }),
      element('p', { text }),
      chips.length ? element('div', { class: 'hero-chips' }, chips) : null,
      actions.length ? element('div', { class: 'page-hero-actions' }, actions) : null,
    ]),
  ]);
}

function keysOf(text) {
  const parts = /^[0-9]/.test(text) ? [text] : text.split('+').map(part => part.trim()).filter(Boolean);
  return element('span', { class: 'keys' }, parts.flatMap((key, index) => [index ? element('span', { class: 'plus', text: '+' }) : null, element('kbd', { text: key })]));
}

function heroChip(text, symbol = null, className = '') {
  return element('span', { class: `hero-chip${className ? ` ${className}` : ''}` }, [symbol ? icon(symbol) : null, element('span', { text })]);
}

function optionRow(name, fallback, symbol, label, description) {
  return row({
    symbol,
    label,
    description,
    control: switchControl(Boolean(getOption(name, fallback)), value => {
      setOption(name, value);
      toast(label, { message: t(value ? 'Activé' : 'Désactivé'), icon: symbol });
    }),
  });
}

function appearancePage() {
  const mode = store.get(keys.mode, 'dark') === 'light' ? 'light' : 'dark';
  const segmented = element('div', { class: 'segmented' });
  const modes = [['dark', 'dark_mode', t('Sombre')], ['light', 'light_mode', t('Clair')]];
  const renderModes = current => segmented.replaceChildren(...modes.map(([id, symbol, label]) => element('button', {
    class: id === current ? 'selected' : '',
    onClick: () => {
      store.set(keys.mode, id);
      applyTheme();
      renderModes(id);
      toast(t('Mode'), { message: label, icon: symbol });
    },
  }, [icon(symbol), element('span', { text: label })])));
  renderModes(mode);

  const languageChoice = getOption('language', 'auto') in languages ? getOption('language', 'auto') : 'auto';
  const languageControl = element('div', { class: 'chips' }, [
    ['auto', t('Automatique')],
    ...Object.entries(languages).map(([id, entry]) => [id, entry.name]),
  ].map(([id, label]) => element('button', {
    class: `chip${id === languageChoice ? ' selected' : ''}`,
    lang: id === 'auto' ? null : id,
    onClick: () => {
      if (id !== getOption('language', 'auto')) setOption('language', id);
    },
  }, [id === languageChoice ? icon('check') : null, element('span', { text: label })])));

  const grid = element('div', { class: 'scheme-grid' });
  const chips = element('div', { class: 'chips' });

  const renderSchemes = () => {
    const selection = store.get(keys.scheme, null);
    const cards = [element('button', {
      class: `scheme${!selection ? ' selected' : ''}`,
      onClick: () => {
        store.set(keys.scheme, null);
        applyTheme();
        renderSchemes();
        toast(t('Schéma de couleurs'), { message: t("Dynamique (fond d'écran)"), icon: 'palette' });
      },
    }, [element('div', { class: 'scheme-swatches dynamic' }, icon('wallpaper')), element('span', { text: t('Dynamique') })])];

    for (const [name, flavours] of Object.entries(schemeBundle ?? {})) {
      for (const [flavour, modesOfScheme] of Object.entries(flavours)) {
        const roles = modesOfScheme.dark ?? modesOfScheme.light;
        const label = `${schemeNames[name] ?? capitalize(name)}${flavour === 'default' || flavour === 'medium' ? '' : ` ${capitalize(flavour)}`}`;
        const active = selection?.name === name && selection?.flavour === flavour;
        cards.push(element('button', {
          class: `scheme${active ? ' selected' : ''}`,
          title: label,
          onClick: () => {
            store.set(keys.scheme, { name, flavour });
            applyTheme();
            renderSchemes();
            toast(t('Schéma de couleurs'), { message: label, icon: 'palette' });
          },
        }, [
          element('div', { class: 'scheme-swatches' }, [roles.primary, roles.secondary, roles.tertiary].map(color => element('span', { style: { background: color } }))),
          element('span', { text: label }),
        ]));
      }
    }
    grid.replaceChildren(...cards);

    const currentVariant = store.get(keys.variant, 'tonalspot');
    chips.replaceChildren(...Object.entries(variants).map(([id, variant]) => element('button', {
      class: `chip${!selection && id === currentVariant ? ' selected' : ''}`,
      onClick: () => {
        store.set(keys.variant, id);
        store.set(keys.scheme, null);
        applyTheme();
        renderSchemes();
        toast(t('Variante dynamique'), { message: variant.label, icon: 'tune' });
      },
    }, [!selection && id === currentVariant ? icon('check') : null, element('span', { text: variant.label })])));
  };
  renderSchemes();

  const selection = store.get(keys.scheme, null);
  const schemeLabel = selection
    ? `${schemeNames[selection.name] ?? capitalize(selection.name)}${selection.flavour === 'default' || selection.flavour === 'medium' ? '' : ` ${capitalize(selection.flavour)}`}`
    : t('Dynamique · {0}', variants[store.get(keys.variant, 'tonalspot')]?.label ?? '');
  const modeLabel = t({ dark: 'Sombre', light: 'Clair' }[mode]);
  return [
    pageHero({
      symbol: 'palette',
      shape: 'flower',
      heading: t('Couleurs et mode'),
      text: t("Une palette Material You complète, tirée de votre fond d'écran ou d'un schéma fixe, appliquée à tout le thème."),
      chips: [
        element('span', { class: 'hero-chip hero-swatches' }, [element('i', { class: 'primary' }), element('i', { class: 'secondary' }), element('i', { class: 'tertiary' })]),
        heroChip(modeLabel, mode === 'light' ? 'light_mode' : 'dark_mode'),
        heroChip(schemeLabel, 'colors'),
      ],
    }),
    section(t('Mode'), [row({ symbol: 'contrast', label: t('Clair ou sombre'), description: t('Choisissez le mode de couleur du thème'), control: segmented })]),
    section(t('Schéma de couleurs'), [grid]),
    section(t('Variante du schéma dynamique'), [
      chips,
      optionRow('smartScheme', true, 'auto_awesome', t('Schéma intelligent'), t("Fond d'écran en noir et blanc : palette monochrome au lieu d'une teinte inventée")),
    ]),
    section(t('Langue'), [row({ symbol: 'language', label: t('Langue du thème'), description: t('Automatique suit la langue d’affichage de Windows') }), languageControl]),
    section(t('Windows'), [
      optionRow('windowsColors', false, 'desktop_windows', t('Accorder Windows au thème'), t("L'accent et le mode clair ou sombre de Windows suivent le thème : Paramètres, Explorateur, menu Démarrer. Vos couleurs d'avant reviennent si vous désactivez.")),
      optionRow('windowsTitleBars', false, 'web_asset', t('Barres de titre colorées'), t("Les barres de titre des fenêtres prennent la couleur d'accent (avec l'option ci-dessus)")),
    ]),
  ];
}

function wallpaperPage() {
  const field = element('input', { class: 'field', type: 'text', spellcheck: 'false', value: getOption('wallpaperDirectory', ''), placeholder: t('Images (dossier par défaut)') });
  const save = element('button', {
    class: 'button filled',
    onClick: () => {
      const value = field.value.trim();
      if (value && !/^[A-Za-z]:\\/.test(value)) {
        toast(t('Dossier invalide'), { message: t('Indique un chemin complet, par exemple C:\\Images'), icon: 'error', kind: 'warning' });
        return;
      }
      setOption('wallpaperDirectory', value);
      toast(t("Dossier des fonds d'écran"), { message: value || t('Images (dossier par défaut)'), icon: 'folder' });
    },
  }, [icon('save'), element('span', { text: t('Enregistrer') })]);

  const sample = store.get(keys.wallpaperSample, null);
  const image = sample ? `url(data:image/png;base64,${sample})` : null;
  const folder = getOption('wallpaperDirectory', '');
  return [
    pageHero({
      symbol: 'wallpaper',
      heading: t('Votre fond, vos couleurs'),
      text: t("Chaque fond d'écran redonne au thème une palette assortie. Wallpaper Engine est pris en compte. Raccourci : Alt+E."),
      art: image ? element('div', { class: 'page-hero-thumb', style: { backgroundImage: image } }) : null,
      backdrop: image ? element('div', { class: 'page-hero-backdrop', style: { backgroundImage: image } }) : null,
      chips: [heroChip(folder.split('\\').filter(Boolean).pop() ?? 'Images', 'folder')],
      actions: [element('button', { class: 'button filled', onClick: async () => { await popup.close(); openLauncher(t('>fond ')); } }, [icon('photo_library'), element('span', { text: t('Choisir un fond') })])],
    }),
    section(t("Dossier des fonds d'écran"), [
      element('div', { class: 'row' }, [field, save]),
      row({
        symbol: 'folder_open',
        label: t('Ouvrir le dossier'),
        description: t("Ajoute ou retire des images, le sélecteur se met à jour tout seul"),
        control: element('button', { class: 'button', onClick: () => openFolder('wallpapers') }, [icon('open_in_new'), element('span', { text: t('Ouvrir') })]),
      }),
    ]),
  ];
}

function desktopPage() {
  const minutes = getOption('idleMinutes', 10);
  const label = element('span', { text: `${minutes} min` });
  const slider = element('input', { type: 'range', min: '1', max: '60', step: '1', value: String(minutes) });
  slider.addEventListener('input', () => { label.textContent = `${slider.value} min`; });
  slider.addEventListener('change', () => setOption('idleMinutes', Number(slider.value)));

  const state = (name, fallback, label) => heroChip(label, getOption(name, fallback) ? 'check_circle' : 'do_not_disturb_on', getOption(name, fallback) ? '' : 'off');
  return [
    pageHero({
      symbol: 'desktop_windows',
      shape: 'gem',
      heading: t('Votre bureau'),
      text: t("Ce que le thème ajoute autour de vos fenêtres : île dynamique, horloge, coin actif et écran d'inactivité."),
      chips: [state('island', true, t('Île dynamique')), state('desktopClock', true, t('Horloge')), state('idleScreen', true, t('Inactivité')), state('gameMode', false, t('Mode jeu'))],
    }),
    section(t('Bureau'), [
      optionRow('hotCorner', true, 'select_window', t('Coin actif'), t('Le coin haut droit ouvre la vue des tâches')),
      optionRow('island', true, 'smart_display', t('Île dynamique'), t('Morceau en cours et notifications en haut de l\'écran')),
      optionRow('tintIcons', true, 'palette', t('Icônes aux couleurs du thème'), t("Teinte les icônes d'applications avec la couleur principale")),
      optionRow('desktopClock', true, 'schedule', t('Horloge de bureau'), t("Grande horloge en bas à droite du fond d'écran")),
      optionRow('clockSeconds', false, 'timer', t('Afficher les secondes'), t('Horloges de la barre et du tableau de bord')),
      optionRow('hideTaskbar', true, 'dock_to_left', t('Remplacer la barre des tâches'), t('Masque la barre Windows : la barre Stellaria liste les fenêtres ouvertes')),
      optionRow('trayCollapse', true, 'unfold_less', t('Replier la zone de notification'), t('Quatre icônes restent visibles, les autres sont derrière une flèche')),
    ]),
    section(t("Écran d'inactivité"), [
      optionRow('idleScreen', true, 'nightlight', t("Afficher l'écran d'inactivité"), t("Horloge, météo et média sur le fond flouté. Le verrouillage de Windows n'est pas modifié.")),
      row({ symbol: 'timer', label: t('Délai avant affichage'), description: t("Jamais pendant une vidéo, une musique ou un jeu en plein écran"), control: element('div', { class: 'range' }, [slider, label]) }),
    ]),
    section(t('Performances'), [
      optionRow('gameMode', false, 'sports_esports', t('Mode jeu'), t('Coupe toutes les animations du thème')),
    ]),
  ];
}

function panelsPage() {
  return [
    pageHero({
      symbol: 'dock_to_left',
      shape: 'softBurst',
      heading: t('Tiroirs et panneaux'),
      text: t("Les tiroirs sortent des bords de l'écran au survol ou au clavier. Choisissez ce qui s'ouvre tout seul."),
      chips: [heroChip(t('Haut : tableau de bord'), 'north'), heroChip(t('Droite : volume'), 'east'), heroChip(t('Coin bas droit : utilitaires'), 'south_east')],
    }),
    section(t('Ouverture au survol'), [
      optionRow('hoverDashboard', true, 'dashboard', t('Tableau de bord'), t("Survol du centre du bord haut")),
      optionRow('hoverOsd', true, 'volume_up', t('Volume et micro'), t('Survol du milieu du bord droit')),
      optionRow('hoverUtilities', true, 'widgets', t('Utilitaires'), t('Survol du coin bas droit')),
      optionRow('hoverPopouts', true, 'left_panel_open', t('Mini-panneaux de la barre'), t('Survol des icônes de la barre')),
    ]),
    section(t('Affichage automatique'), [
      optionRow('osdOnVolume', true, 'tune', t('Volume à chaque changement'), t("Montre l'indicateur quand le volume change")),
      optionRow('toasts', true, 'notifications', t('Notifications du thème'), t('Petites confirmations en bas à droite')),
      optionRow('systemToasts', true, 'notifications_active', t('Événements système'), t('Sortie et entrée audio, disposition du clavier, Verr. Maj / Num, morceau en cours')),
    ]),
    section(t('Tableau de bord'), [
      optionRow('weather', true, 'partly_cloudy_day', t('Météo'), t("Prévisions d'Open-Meteo pour votre position approximative (ipinfo.io). Désactivée, rien n'est envoyé.")),
      optionRow('lyrics', true, 'lyrics', t('Paroles synchronisées'), t("Onglet Média, paroles fournies par LRCLIB quand elles existent. Désactivées, rien n'est envoyé.")),
    ]),
    section(t('Lanceur'), [hiddenAppsList()]),
  ];
}

function hiddenAppsList() {
  const container = element('div', { class: 'hidden-apps' });
  const renderList = () => {
    const hidden = getOption('hiddenApps', []);
    const names = new Map((store.get(keys.apps)?.items ?? []).map(app => [app.id, app.name]));
    if (!hidden.length) {
      container.replaceChildren(row({ symbol: 'star', label: t('Favoris et applications masquées'), description: t("Dans le lanceur : l'étoile ou Ctrl+F épingle une application en tête, l'œil barré ou Ctrl+H la masque") }));
      return;
    }
    container.replaceChildren(...hidden.map(id => row({
      symbol: 'visibility_off',
      label: names.get(id) ?? id,
      description: t('Masquée du lanceur'),
      control: element('button', {
        class: 'button',
        onClick: () => {
          setOption('hiddenApps', getOption('hiddenApps', []).filter(hiddenId => hiddenId !== id));
          toast(t('Application réaffichée'), { message: names.get(id) ?? id, icon: 'visibility' });
          renderList();
        },
      }, [icon('visibility'), element('span', { text: t('Réafficher') })]),
    })));
  };
  renderList();
  return container;
}

function shortcutsPage() {
  const shortcuts = [
    ['Alt+Espace', 'Lanceur d’applications'],
    ['Alt+E', "Fonds d'écran"],
    ['Win+Alt+A', 'Tableau de bord'],
    ['Win+Alt+U', 'Utilitaires'],
    ['Win+Alt+V', 'Historique du presse-papiers'],
    ['Win+Alt+.', 'Emojis'],
    ['Win+Alt+P', 'Menu de session'],
    ['Win+Alt+S', 'Réglages du thème'],
  ].map(([combo, label]) => [t(combo), t(label)]);
  const prefixes = [
    ['>', 'Actions'],
    ['>fond', "Fonds d'écran"],
    ['>schéma', 'Schémas de couleurs'],
    ['>variante', 'Variantes dynamiques'],
    ['>clip', 'Presse-papiers'],
    [':', 'Emojis'],
    ['2+2', 'Calculatrice'],
  ].map(([combo, label]) => [t(combo), t(label)]);
  const grid = list => element('div', { class: 'shortcuts' }, list.map(([keysText, label]) => element('div', { class: 'shortcut' }, [
    keysOf(keysText),
    element('span', { class: 'shortcut-label', text: label }),
  ])));
  return [
    pageHero({
      symbol: 'keyboard',
      shape: 'cookie4',
      heading: t('Tout au clavier'),
      text: t('Chaque panneau a son raccourci. Un appui sur Win ouvre le lanceur, Win+Alt+K affiche cette aide partout.'),
      chips: [heroChip(t('Win : lanceur'), 'keyboard_command_key'), heroChip(t('Win+Alt+K : aide'), 'help')],
    }),
    section(t('Raccourcis globaux'), [grid(shortcuts)]),
    section(t('Dans le lanceur'), [grid(prefixes)]),
  ];
}

function welcomePage() {
  const step = (number, label, children) => element('div', { class: 'welcome-step' }, [
    element('span', { class: 'welcome-number', text: String(number) }),
    element('div', { class: 'welcome-body' }, [element('span', { class: 'section-title', text: label }), ...children]),
  ]);
  const essentials = [
    ['Win', 'Lanceur'],
    ['Win + Alt + A', 'Tableau de bord'],
    ['Win + Alt + U', 'Utilitaires'],
    ['Win + Alt + K', 'Tous les raccourcis'],
  ].map(([combo, label]) => [combo, t(label)]);
  return [
    pageHero({
      symbol: 'auto_awesome',
      heading: t('Bienvenue dans Stellaria'),
      text: t('Quatre étapes rapides pour régler le thème à votre goût. Tout reste modifiable plus tard dans ces réglages.'),
    }),
    step(1, t("Fond d'écran"), [
      element('p', { class: 'welcome-text', text: t("Les couleurs de tout le thème viennent de votre fond d'écran.") }),
      element('button', { class: 'button', onClick: async () => { await popup.close(); openLauncher(t('>fond ')); } }, [icon('photo_library'), element('span', { text: t('Choisir un fond') })]),
    ]),
    step(2, t('Raccourcis essentiels'), [
      element('div', { class: 'welcome-keys' }, essentials.map(([keys, label]) => element('div', { class: 'welcome-key' }, [
        keysOf(keys),
        element('span', { text: label }),
      ]))),
    ]),
    step(3, t('Apparence'), [
      optionRow('tintIcons', true, 'palette', t('Icônes aux couleurs du thème'), t("Teinte les icônes d'applications")),
      optionRow('island', true, 'smart_display', t('Île dynamique'), t('Morceau en cours et notifications en haut de l\'écran')),
    ]),
    step(4, t('Démarrage'), [
      element('p', { class: 'welcome-text', text: t("Stellaria se lance à chaque ouverture de session. En cas de souci, le menu Démarrer contient « Stellaria - Rétablir la barre des tâches ».") }),
      element('button', {
        class: 'button primary',
        onClick: () => {
          setOption('welcomed', true);
          showPage('appearance');
        },
      }, [icon('check'), element('span', { text: t("C'est parti") })]),
    ]),
  ];
}

let helperConnected = false;
helperClient.onConnection(connected => {
  helperConnected = connected;
  document.querySelectorAll('.about-status').forEach(paintStatus);
});

function paintStatus(node) {
  node.classList.toggle('online', helperConnected);
  node.lastChild.textContent = t(helperConnected ? 'Service connecté' : 'Service hors ligne');
}

let packVersion = '';
fetch('./zpack.json').then(response => response.json()).then(pack => {
  packVersion = pack.version ?? '';
  document.querySelectorAll('.about-version').forEach(node => { node.textContent = t('Version {0}', packVersion); });
}).catch(() => {});

function aboutPage() {
  const status = element('span', { class: 'hero-chip about-status' }, [element('span', { class: 'about-dot' }), element('span', {})]);
  paintStatus(status);
  const highlight = (symbol, label, text) => element('div', { class: 'about-tile' }, [
    element('span', { class: 'about-tile-icon' }, icon(symbol)),
    element('span', { class: 'about-tile-title', text: label }),
    element('span', { class: 'about-tile-text', text }),
  ]);
  const credit = (symbol, label, text) => element('div', { class: 'about-credit' }, [
    element('span', { class: 'about-credit-icon' }, icon(symbol)),
    element('div', { class: 'row-text' }, [
      element('span', { class: 'row-title', text: label }),
      element('span', { class: 'row-subtitle', text }),
    ]),
  ]);
  const stack = ['Zebar', '.NET 10', 'Material Color Utilities', 'Open-Meteo', 'LRCLIB', 'Google Sans Flex', 'Rubik', 'Material Symbols'];

  return [
    pageHero({
      symbol: 'auto_awesome',
      heading: t('Stellaria pour Windows'),
      text: t('Stellaria recrée le shell Material You de Caelestia pour Windows 11 : barre, tiroirs et animations expressives autour de vos fenêtres habituelles.'),
      chips: [
        element('span', { class: 'hero-chip about-version', text: packVersion ? t('Version {0}', packVersion) : t('Version') }),
        element('span', { class: 'hero-chip', text: t('Licence GPL-3.0') }),
        status,
      ],
    }),
    element('div', { class: 'about-tiles' }, [
      highlight('palette', 'Material You', t("Une palette complète tirée de votre fond d'écran, en clair comme en sombre.")),
      highlight('animation', t('Mouvement expressif'), t('Les courbes et les rebonds de Material 3 Expressive, fluides à chaque ouverture.')),
      highlight('desktop_windows', t('Pensé pour Windows'), t('Fenêtres, raccourcis et notifications Windows restent comme vous les connaissez.')),
    ]),
    section(t('Crédits'), [
      credit('favorite', 'Caelestia', t("soramanew et les contributeurs : design, couleurs et animations d'origine")),
      credit('widgets', 'Zebar', t('glzr-io : moteur des widgets')),
      credit('lightbulb', t('Inspirations'), "DankMaterialShell, Noctalia, end-4's dots, YASB, FluentFlyout"),
    ]),
    section(t('Construit avec'), [
      element('div', { class: 'about-stack' }, stack.map(name => element('span', { class: 'hero-chip quiet', text: name }))),
    ]),
    element('div', { class: 'about-actions' }, [
      element('button', { class: 'button', onClick: () => showPage('welcome') }, [icon('waving_hand'), element('span', { text: t("Revoir l'accueil") })]),
      element('button', { class: 'button', onClick: () => showPage('shortcuts') }, [icon('keyboard'), element('span', { text: t('Raccourcis') })]),
    ]),
  ];
}

/* STELLARIA CUSTOMIZER SETTINGS PAGE START */
function customizerConfig() {
  return Object.fromEntries(Object.entries(customizerDefaults).map(([key, fallback]) => [key, getOption(key, fallback)]));
}

async function applyCustomizations(draft, button) {
  if (button?.disabled) return;
  if (button) button.disabled = true;
  toast(t('Personnalisations Stellaria'), { message: t('Application et rechargement de l’interface…'), icon: 'sync' });
  try {
    for (const [key, value] of Object.entries(draft)) setOption(key, value);
    await helperClient.request('config.set', { path: 'launcher.winKey', value: draft.launcherWinKey });
    toast(t('Personnalisations Stellaria'), { message: t('Préférences appliquées.'), icon: 'check_circle' });
    await helperClient.request('widgets.reload');
  } catch (error) {
    if (button) button.disabled = false;
    console.error('[stellaria] customizations', error);
    toast(t('Impossible d’appliquer'), { message: String(error?.message || error), icon: 'error', kind: 'warning' });
  }
}

function customizerToggle(draft, key, symbol, label, description) {
  return row({
    symbol,
    label,
    description,
    control: switchControl(Boolean(draft[key]), value => { draft[key] = value; }),
  });
}

function customizerPage() {
  const draft = customizerConfig();
  const borderLabel = element('span', { class: 'customizer-value', text: `${draft.borderSize} px` });
  const borderSlider = element('input', {
    class: 'customizer-slider',
    type: 'range',
    min: '0',
    max: '40',
    step: '1',
    value: String(draft.borderSize),
  });
  borderSlider.addEventListener('input', () => {
    draft.borderSize = Number(borderSlider.value);
    borderLabel.textContent = `${draft.borderSize} px`;
  });

  const applyButton = element('button', {
    class: 'button filled customizer-apply',
    onClick: () => applyCustomizations(draft, applyButton),
  }, [icon('save'), element('span', { text: t('Appliquer et recharger') })]);

  return [
    pageHero({
      symbol: 'tune',
      shape: 'gem',
      heading: t('Personnalisations Stellaria'),
      text: t('Contrôlez les préférences intégrées. L’interface sera rechargée à l’application.'),
      chips: [
        heroChip(t('4 écrans'), 'monitor'),
        heroChip(t(draft.groupTasks ? 'Applications groupées' : 'Fenêtres séparées'), 'select_window'),
        heroChip(`${draft.borderSize} px`, 'border_style'),
      ],
    }),
    section(t('Barre des tâches et écrans'), [
      customizerToggle(draft, 'showActiveWindow', 'title', t('Afficher la fenêtre active'), t('Affiche à nouveau le nom et l’icône de la fenêtre active dans la barre.')),
      customizerToggle(draft, 'showTasksFromAllMonitors', 'monitor', t('Fenêtres de tous les écrans'), t('Affiche la liste globale des applications dans chacune des quatre barres.')),
      customizerToggle(draft, 'groupTasks', 'tab_group', t('Grouper les fenêtres de la même application'), t('Combine plusieurs fenêtres dans une icône avec compteur et sélecteur.')),
    ]),
    section(t('Aperçus et comportement'), [
      customizerToggle(draft, 'taskPreviews', 'preview', t('Aperçu au survol'), t('Affiche aussi une miniature lorsque l’application ne possède qu’une fenêtre.')),
      customizerToggle(draft, 'minimizeActiveTaskOnClick', 'minimize', t('Réduire en cliquant sur la fenêtre active'), t('Désactivé, conserve le focus et évite que la fenêtre se réduise de façon inattendue.')),
    ]),
    section(t('Bordures'), [
      row({
        symbol: 'border_style',
        label: t('Épaisseur du cadre'),
        description: t('0 supprime le cadre sans réserver d’espace ; les déclencheurs invisibles restent à 1 px.'),
        control: element('div', { class: 'customizer-range' }, [borderSlider, borderLabel]),
      }),
    ]),
    section(t('Windows et lanceur'), [
      customizerToggle(draft, 'launcherWinKey', 'keyboard_command_key', t('La touche Windows ouvre le lanceur'), t('Désactivé, conserve le menu Démarrer natif de Windows.')),
    ]),
    section(t('Application et récupération'), [
      element('div', { class: 'customizer-actions' }, [applyButton]),
      element('p', { class: 'customizer-note', text: t('Ces préférences font partie de cette version de Stellaria et ne nécessitent pas de configurateur séparé.') }),
    ]),
  ];
}
/* STELLARIA CUSTOMIZER SETTINGS PAGE END */

const pages = [
  ['welcome', 'waving_hand', 'Bienvenue', welcomePage],
  ['appearance', 'palette', 'Apparence', appearancePage],
  ['wallpaper', 'wallpaper', "Fond d'écran", wallpaperPage],
  ['desktop', 'desktop_windows', 'Bureau', desktopPage],
  ['panels', 'dock_to_left', 'Panneaux', panelsPage],
  ['customizer', 'tune', 'Personnalisations', customizerPage],
  ['shortcuts', 'keyboard', 'Raccourcis', shortcutsPage],
  ['about', 'info', 'À propos', aboutPage],
].map(([id, symbol, label, build]) => [id, symbol, t(label), build]);

const rail = document.getElementById('rail');
const brand = element('div', { class: 'rail-brand' }, [shapeSvg('sunny'), icon('auto_awesome')]);

function showPage(id) {
  const [, , label, build] = pages.find(page => page[0] === id);
  title.textContent = label;
  rail.querySelectorAll('.rail-item').forEach(item => item.classList.toggle('current', item.dataset.page === id));
  pagesNode.replaceChildren(element('div', { class: 'page' }, build()));
  pagesNode.scrollTop = 0;
}

rail.replaceChildren(brand, ...pages.map(([id, symbol, label]) => element('button', { class: 'rail-item', 'data-page': id, onClick: () => showPage(id) }, [
  element('span', { class: 'rail-pill' }, icon(symbol)),
  element('span', { text: label }),
])));

showPage('appearance');

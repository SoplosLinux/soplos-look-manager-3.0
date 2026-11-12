// soplos-look-manager-style-preview
//
// Draws the pictures of the pages of System Settings that Plasma draws with code
// of its own and does not keep as files, using the same libraries and the same
// calls as those pages, and saves them as PNG files for Soplos Look Manager.
//
//   soplos-look-manager-style-preview style <style> <output.png>
//       Controls drawn with a Qt widget style and the color scheme in use, as
//       the Application Style page does (kcms/style/previewitem.cpp).
//
//   soplos-look-manager-style-preview plasma <theme> <output.png>
//       The frame and the analog clock of a Plasma theme, with KSvg, as the
//       Plasma Style page does (kcms/desktoptheme/ui/ThemePreview.qml).
//
//   soplos-look-manager-style-preview decoration <plugin> <theme> <caption> <output.png>
//       A window decoration, loaded from its plugin of KWin and painted around
//       an empty window, as the Window Decorations page does
//       (kcms/decoration/declarative-plugin/previewitem.cpp). The caption is the
//       name the windows carry (empty to take it from the theme).
//
// Exit codes: 0 done, 1 wrong usage, 2 the style, theme or plugin does not exist,
// 3 the picture could not be saved.

#include <QApplication>
#include <QCheckBox>
#include <QComboBox>
#include <QGridLayout>
#include <QHBoxLayout>
#include <QFile>
#include <QIcon>
#include <QImage>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QPainter>
#include <QProgressBar>
#include <QQuickItem>
#include <QQuickWindow>
#include <QSGRendererInterface>
#include <QPushButton>
#include <QRadioButton>
#include <QScrollBar>
#include <QSlider>
#include <QSpinBox>
#include <QDir>
#include <QStandardPaths>
#include <QStyle>
#include <QStyleFactory>
#include <QTabWidget>
#include <QVBoxLayout>
#include <QWidget>

#include <KColorScheme>
#include <KConfigGroup>
#include <KDecoration3/DecoratedWindow>
#include <KDecoration3/Decoration>
#include <KDecoration3/DecorationSettings>
#include <KDecoration3/Private/DecoratedWindowPrivate>
#include <KDecoration3/Private/DecorationBridge>
#include <KDecoration3/Private/DecorationSettingsPrivate>
#include <KPackage/PackageLoader>
#include <KPluginFactory>
#include <KPluginMetaData>
#include <KSharedConfig>
#include <KSvg/FrameSvg>
#include <KSvg/ImageSet>
#include <KSvg/Svg>

#include <algorithm>
#include <cmath>
#include <cstdio>
#include <memory>

namespace {

// The size of every picture: the shape of a card of the list, which is wide (16:9)
// as the cards of the themes and the ones of System Settings.
constexpr int kCardWidth = 320;
constexpr int kCardHeight = 180;
constexpr int kStyleHeight = kCardHeight;

// The texts of the controls of the preview, in the language of the program (the
// program passes it in SOPLOS_PREVIEW_LANG). The page of System Settings shows
// them in the language of the desktop.
QString controlText(const char *key)
{
    struct Row {
        const char *lang;
        const char *tab1, *tab2, *checkbox, *radio, *combo, *button;
    };
    static const Row rows[] = {
        {"en", "Tab 1", "Tab 2", "Checkbox", "Radio button", "Combo box", "Push button"},
        {"es", "Pestaña 1", "Pestaña 2", "Casilla de selección", "Botón de selección", "Lista desplegable", "Botón"},
        {"de", "Reiter 1", "Reiter 2", "Kontrollkästchen", "Auswahlknopf", "Auswahlliste", "Knopf"},
        {"fr", "Onglet 1", "Onglet 2", "Case à cocher", "Bouton radio", "Liste déroulante", "Bouton"},
        {"it", "Scheda 1", "Scheda 2", "Casella di controllo", "Pulsante di opzione", "Elenco a discesa", "Pulsante"},
        {"pt", "Separador 1", "Separador 2", "Caixa de verificação", "Botão de opção", "Lista pendente", "Botão"},
        {"ro", "Fila 1", "Fila 2", "Casetă de bifare", "Buton radio", "Listă derulantă", "Buton"},
        {"ru", "Вкладка 1", "Вкладка 2", "Флажок", "Переключатель", "Выпадающий список", "Кнопка"},
    };
    const QString lang = qEnvironmentVariable("SOPLOS_PREVIEW_LANG", QStringLiteral("en")).left(2);
    const Row *row = &rows[0];
    for (const Row &candidate : rows) {
        if (lang == QLatin1String(candidate.lang)) {
            row = &candidate;
        }
    }
    const QString name = QString::fromLatin1(key);
    const char *text = name == QLatin1String("tab1")       ? row->tab1
                       : name == QLatin1String("tab2")     ? row->tab2
                       : name == QLatin1String("checkbox") ? row->checkbox
                       : name == QLatin1String("radio")    ? row->radio
                       : name == QLatin1String("combo")    ? row->combo
                                                           : row->button;
    return QString::fromUtf8(text);
}

// The file that has the colors the pictures are drawn with: the color scheme in
// use, or the scheme that SOPLOS_PREVIEW_SCHEME names (a `.colors` file), which is
// how the program asks for the pictures of its light or dark theme.
KSharedConfigPtr schemeConfig()
{
    const QString file = qEnvironmentVariable("SOPLOS_PREVIEW_SCHEME");
    if (!file.isEmpty() && QFile::exists(file)) {
        return KSharedConfig::openConfig(file, KConfig::SimpleConfig);
    }
    return KSharedConfig::openConfig(QStringLiteral("kdeglobals"));
}

// The colors of the scheme, as the pages of System Settings read them.
QPalette schemePalette()
{
    return KColorScheme::createApplicationPalette(schemeConfig());
}

// ── Application styles ───────────────────────────────────────────────────────

// Gives the style and the palette to a widget and to everything inside it.
void applyStyleRecursively(QWidget *widget, QStyle *style, const QPalette &palette)
{
    widget->setPalette(palette);
    widget->setStyle(style);
    const auto children = widget->findChildren<QWidget *>(Qt::FindDirectChildrenOnly);
    for (QWidget *child : children) {
        applyStyleRecursively(child, style, palette);
    }
}

int drawStyle(const QString &styleName, const QString &output)
{
    std::unique_ptr<QStyle> style(QStyleFactory::create(styleName));
    if (!style) {
        return 2;
    }

    QPalette palette = schemePalette();
    style->polish(palette);
    // The preview has no window to be active, so it is drawn as an active one.
    for (int role = 0; role < QPalette::NColorRoles; ++role) {
        palette.setBrush(QPalette::Inactive, static_cast<QPalette::ColorRole>(role), palette.brush(QPalette::Active, static_cast<QPalette::ColorRole>(role)));
    }

    int result = 3;
    {
        QWidget preview;
        preview.setAttribute(Qt::WA_DontShowOnScreen);

        // The widgets of kcms/style/stylepreview.ui: a tab widget whose first tab
        // has a check box, two radio buttons, a combo box, a push button, a spin
        // box, a slider, a progress bar and a vertical scroll bar.
        auto *root = new QHBoxLayout(&preview);
        root->setContentsMargins(2, 2, 2, 2);
        auto *tabs = new QTabWidget(&preview);
        tabs->addTab(new QWidget(tabs), controlText("tab1"));
        tabs->addTab(new QWidget(tabs), controlText("tab2"));
        root->addWidget(tabs);

        QWidget *page = tabs->widget(0);
        auto *grid = new QGridLayout(page);

        auto *column = new QVBoxLayout();
        auto *check = new QCheckBox(controlText("checkbox"), page);
        check->setChecked(true);
        auto *radioOn = new QRadioButton(controlText("radio"), page);
        radioOn->setChecked(true);
        auto *radioOff = new QRadioButton(controlText("radio"), page);
        column->addWidget(check);
        column->addWidget(radioOn);
        column->addWidget(radioOff);

        auto *combo = new QComboBox(page);
        combo->addItem(controlText("combo"));
        combo->setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Fixed);
        auto *scroll = new QScrollBar(Qt::Vertical, page);
        scroll->setMaximum(10);
        auto *button = new QPushButton(controlText("button"), page);
        button->setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Fixed);
        auto *spin = new QSpinBox(page);
        spin->setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Fixed);
        auto *slider = new QSlider(Qt::Horizontal, page);
        slider->setValue(30);
        auto *progress = new QProgressBar(page);
        progress->setValue(70);

        grid->addLayout(column, 6, 0, 2, 1);
        grid->addWidget(combo, 6, 1, 1, 2);
        grid->addWidget(scroll, 6, 3, 4, 1);
        grid->addWidget(button, 7, 1);
        grid->addWidget(spin, 7, 2);
        grid->addWidget(slider, 8, 0);
        grid->addWidget(progress, 8, 1, 1, 2);

        applyStyleRecursively(&preview, style.get(), palette);

        preview.setAutoFillBackground(true);
        preview.ensurePolished();
        preview.resize(preview.sizeHint().expandedTo(QSize(360, 172)));
        preview.show();

        // The controls are put on a card of the color of the window, as big as the
        // card allows with almost no margin, so the picture fills it.
        const QPixmap controls = preview.grab();
        QImage card(kCardWidth, kStyleHeight, QImage::Format_ARGB32_Premultiplied);
        card.fill(palette.color(QPalette::Window));
        {
            QPainter painter(&card);
            painter.setRenderHint(QPainter::SmoothPixmapTransform);
            const QPixmap fitted = controls.scaled(QSize(kCardWidth - 8, kStyleHeight - 8), Qt::KeepAspectRatio, Qt::SmoothTransformation);
            painter.drawPixmap((kCardWidth - fitted.width()) / 2, (kStyleHeight - fitted.height()) / 2, fitted);
        }
        if (card.save(output, "PNG")) {
            result = 0;
        }
    }

    // The widgets are gone before the style that they used.
    return result;
}

// ── Plasma themes ────────────────────────────────────────────────────────────

int drawPlasma(const QString &themeName, const QString &output)
{
    // The theme is found in the folders `plasma/desktoptheme` of the data
    // folders, and what it does not bring comes from the default theme.
    KSvg::ImageSet imageSet(themeName, QStringLiteral("plasma/desktoptheme"));
    if (imageSet.imageSetName() != themeName) {
        return 2;
    }

    KSvg::FrameSvg frame;
    frame.setUsingRenderingCache(false);
    frame.setCacheAllRenderedFrames(false);
    frame.setImageSet(&imageSet);
    frame.setImagePath(QStringLiteral("widgets/background"));
    if (!frame.isValid()) {
        return 2;
    }

    KSvg::Svg clock;
    clock.setUsingRenderingCache(false);
    clock.setImageSet(&imageSet);
    clock.setImagePath(QStringLiteral("widgets/clock"));

    // The page draws the frame on the color of the window, with the frame as big
    // as the preview.
    QImage image(kCardWidth, kCardHeight, QImage::Format_ARGB32_Premultiplied);
    image.fill(schemePalette().color(QPalette::Window));
    {
        QPainter painter(&image);
        painter.setRenderHint(QPainter::Antialiasing);
        painter.setRenderHint(QPainter::SmoothPixmapTransform);

        // The frame is drawn so that the visible part of it (without the inset
        // that its shadow needs) fills the card.
        const qreal pad = 6;
        const qreal insetLeft = frame.insetSize(KSvg::FrameSvg::LeftMargin);
        const qreal insetTop = frame.insetSize(KSvg::FrameSvg::TopMargin);
        const qreal insetRight = frame.insetSize(KSvg::FrameSvg::RightMargin);
        const qreal insetBottom = frame.insetSize(KSvg::FrameSvg::BottomMargin);
        frame.resizeFrame(QSizeF(kCardWidth - 2 * pad + insetLeft + insetRight, kCardHeight - 2 * pad + insetTop + insetBottom));
        frame.paintFrame(&painter, QPointF(pad - insetLeft, pad - insetTop));

        if (clock.isValid()) {
            // The clock of ThemePreview.qml and Hand.qml: the face is a square as big
            // as the preview allows, every other element is drawn at its natural size
            // times the scale of the face, and the hands turn on their rotation
            // center (the hint of the theme, or the middle of the element).
            clock.resize();
            const qreal side = kCardHeight - 2 * pad - 14;
            const QSizeF naturalFace = clock.elementSize(QStringLiteral("ClockFace"));
            const qreal scale = naturalFace.width() > 0 ? side / naturalFace.width() : 1.0;
            const QPointF center(kCardWidth / 2.0, kCardHeight / 2.0);
            const QRectF faceRect(center - QPointF(side / 2, side / 2), QSizeF(side, side));

            // The size of an element on the preview, rounded to an even number as the page does.
            const auto scaled = [&](const QSizeF &natural) {
                const qreal w = std::round(natural.width() * scale);
                const qreal h = std::round(natural.height() * scale);
                return QSizeF(w + std::fmod(w, 2.0), h + std::fmod(w, 2.0));
            };

            clock.paint(&painter, faceRect, QStringLiteral("ClockFace"));

            const auto hand = [&](const QString &id, const QString &hint, qreal angle) {
                const QRectF natural = clock.elementRect(id);
                const QSizeF size = scaled(natural.size());
                // Hand.qml: with no hint in the theme, the rotation center is
                // width / 2 on both axes, where the hub of the hand is.
                QPointF pivot(size.width() / 2, size.width() / 2);
                if (clock.hasElement(hint)) {
                    const QRectF hinted = clock.elementRect(hint);
                    pivot = QPointF(std::round((hinted.x() - natural.x() + hinted.width() / 2) * scale), std::round((hinted.y() - natural.y() + hinted.height() / 2) * scale));
                }
                painter.save();
                painter.translate(center);
                painter.rotate(angle);
                painter.translate(-center);
                clock.paint(&painter, QRectF(center - pivot, size), id);
                painter.restore();
            };

            // The hands are at 9:05, as the page has them.
            hand(QStringLiteral("HourHand"), QStringLiteral("hint-hourhand-rotation-center-offset"), 180 + 9 * 30 + 5 / 2.0);
            hand(QStringLiteral("MinuteHand"), QStringLiteral("hint-minutehand-rotation-center-offset"), 180 + 5 * 6);

            const QSizeF screw = scaled(clock.elementSize(QStringLiteral("HandCenterScrew")));
            clock.paint(&painter, QRectF(center - QPointF(screw.width() / 2, screw.height() / 2), screw), QStringLiteral("HandCenterScrew"));
            clock.paint(&painter, faceRect, QStringLiteral("Glass"));
        }
    }
    return image.save(output, "PNG") ? 0 : 3;
}

// ── Window decorations ───────────────────────────────────────────────────────

// The title of the windows of the preview: the page puts the name of the
// decoration on them.
QString g_caption = QStringLiteral("Window");

// The colors of the title bar, from the group WM of the color scheme in use.
class DecorationColors
{
public:
    DecorationColors()
    {
        const QPalette palette = schemePalette();
        const KConfigGroup wm(schemeConfig(), QStringLiteral("WM"));
        m_activeBackground = wm.readEntry("activeBackground", palette.color(QPalette::Active, QPalette::Highlight));
        m_activeForeground = wm.readEntry("activeForeground", palette.color(QPalette::Active, QPalette::HighlightedText));
        m_inactiveBackground = wm.readEntry("inactiveBackground", palette.color(QPalette::Active, QPalette::Window));
        m_inactiveForeground = wm.readEntry("inactiveForeground", palette.color(QPalette::Disabled, QPalette::WindowText));
        m_activeFrame = wm.readEntry("frame", m_activeBackground);
        m_inactiveFrame = wm.readEntry("inactiveFrame", m_inactiveBackground);
    }

    QColor color(KDecoration3::ColorGroup group, KDecoration3::ColorRole role) const
    {
        const bool active = group == KDecoration3::ColorGroup::Active;
        switch (role) {
        case KDecoration3::ColorRole::Foreground:
            return active ? m_activeForeground : m_inactiveForeground;
        case KDecoration3::ColorRole::TitleBar:
            return active ? m_activeBackground : m_inactiveBackground;
        case KDecoration3::ColorRole::Frame:
            return active ? m_activeFrame : m_inactiveFrame;
        }
        return QColor();
    }

private:
    QColor m_activeBackground, m_activeForeground, m_inactiveBackground, m_inactiveForeground, m_activeFrame, m_inactiveFrame;
};

// The window of the preview, as the Window Decorations page gives it to the
// decoration: active, with every button available and the size of the client
// area that is asked for.
class PreviewClient : public KDecoration3::DecoratedWindowPrivateV4
{
public:
    PreviewClient(KDecoration3::DecoratedWindow *window, KDecoration3::Decoration *decoration, const QPalette &palette, const QSizeF &size, bool active)
        : KDecoration3::DecoratedWindowPrivateV4(window, decoration)
        , m_palette(palette)
        , m_size(size)
        , m_active(active)
    {
    }

    // The page sets the size of the client when it knows the borders of the
    // decoration, and the decoration is told that it changed.
    void setSize(const QSizeF &size)
    {
        if (size == m_size) {
            return;
        }
        m_size = size;
        Q_EMIT window()->widthChanged(size.width());
        Q_EMIT window()->heightChanged(size.height());
        Q_EMIT window()->sizeChanged(size);
    }

    bool isActive() const override { return m_active; }
    QString caption() const override { return g_caption; }
    bool isOnAllDesktops() const override { return false; }
    bool isShaded() const override { return false; }
    QIcon icon() const override { return QIcon::fromTheme(QStringLiteral("start-here-kde")); }
    bool isMaximized() const override { return false; }
    bool isMaximizedHorizontally() const override { return false; }
    bool isMaximizedVertically() const override { return false; }
    bool isKeepAbove() const override { return false; }
    bool isKeepBelow() const override { return false; }
    bool isCloseable() const override { return true; }
    bool isMaximizeable() const override { return true; }
    bool isMinimizeable() const override { return true; }
    bool providesContextHelp() const override { return false; }
    bool isModal() const override { return false; }
    bool isShadeable() const override { return false; }
    bool isMoveable() const override { return true; }
    bool isResizeable() const override { return true; }
    qreal width() const override { return m_size.width(); }
    qreal height() const override { return m_size.height(); }
    QSizeF size() const override { return m_size; }
    QPalette palette() const override { return m_palette; }
    Qt::Edges adjacentScreenEdges() const override { return {}; }
    qreal scale() const override { return 1.0; }
    qreal nextScale() const override { return 1.0; }
    void requestShowToolTip(const QString &) override {}
    void requestHideToolTip() override {}
    void requestClose() override {}
    void requestToggleMaximization(Qt::MouseButtons) override {}
    void requestMinimize() override {}
    void requestContextHelp() override {}
    void requestToggleOnAllDesktops() override {}
    void requestToggleShade() override {}
    void requestToggleKeepAbove() override {}
    void requestToggleKeepBelow() override {}
    void requestShowWindowMenu(const QRect &) override {}
    QString windowClass() const override { return QString(); }
    bool hasApplicationMenu() const override { return false; }
    bool isApplicationMenuActive() const override { return false; }
    void showApplicationMenu(int) override {}
    void requestShowApplicationMenu(const QRect &, int) override {}
    QString applicationMenuServiceName() const override { return QString(); }
    QString applicationMenuObjectPath() const override { return QString(); }
    void popup(const KDecoration3::Positioner &, QMenu *) override {}
    bool isExcludedFromCapture() const override { return false; }
    void requestToggleExcludeFromCapture() override {}

    QColor color(KDecoration3::ColorGroup group, KDecoration3::ColorRole role) const override { return m_colors.color(group, role); }

private:
    QPalette m_palette;
    QSizeF m_size;
    DecorationColors m_colors;
    bool m_active;
};

// The settings of the decoration: the buttons System Settings shows by default.
class PreviewSettings : public KDecoration3::DecorationSettingsPrivateV2
{
public:
    explicit PreviewSettings(KDecoration3::DecorationSettings *parent)
        : KDecoration3::DecorationSettingsPrivateV2(parent)
    {
    }

    bool isOnAllDesktopsAvailable() const override { return true; }
    bool isAlphaChannelSupported() const override { return true; }
    bool isCloseOnDoubleClickOnMenu() const override { return false; }
    QList<KDecoration3::DecorationButtonType> decorationButtonsLeft() const override
    {
        return {KDecoration3::DecorationButtonType::Menu, KDecoration3::DecorationButtonType::OnAllDesktops};
    }
    QList<KDecoration3::DecorationButtonType> decorationButtonsRight() const override
    {
        return {KDecoration3::DecorationButtonType::Minimize, KDecoration3::DecorationButtonType::Maximize, KDecoration3::DecorationButtonType::Close};
    }
    KDecoration3::BorderSize borderSize() const override { return KDecoration3::BorderSize::Normal; }
    bool isAlwaysShowExcludeFromCapture() const override { return false; }
};

class PreviewBridge : public KDecoration3::DecorationBridge
{
public:
    PreviewBridge(const QPalette &palette, const QSizeF &size, bool active)
        : m_palette(palette)
        , m_size(size)
        , m_active(active)
    {
    }

    // The client that the decoration got, to give it its size afterwards.
    PreviewClient *lastCreatedClient() const { return m_client; }

    std::unique_ptr<KDecoration3::DecoratedWindowPrivate> createClient(KDecoration3::DecoratedWindow *window, KDecoration3::Decoration *decoration) override
    {
        auto client = std::make_unique<PreviewClient>(window, decoration, m_palette, m_size, m_active);
        m_client = client.get();
        return client;
    }

    std::unique_ptr<KDecoration3::DecorationSettingsPrivate> settings(KDecoration3::DecorationSettings *parent) override
    {
        return std::make_unique<PreviewSettings>(parent);
    }

private:
    QPalette m_palette;
    QSizeF m_size;
    bool m_active;
    PreviewClient *m_client = nullptr;
};

// One decoration of the preview, with what it needs to live: the bridge, the
// settings and the decoration, which go away in that order reversed.
struct PreviewDecoration {
    std::unique_ptr<PreviewBridge> bridge;
    std::shared_ptr<KDecoration3::DecorationSettings> settings;
    std::unique_ptr<KDecoration3::Decoration> decoration;
};

// The same calls, in the same order, as the Window Decorations page.
PreviewDecoration makeDecoration(KPluginFactory *factory, const QString &theme, const QPalette &palette, bool active, QQuickItem *visualParent = nullptr)
{
    PreviewDecoration made;
    made.bridge = std::make_unique<PreviewBridge>(palette, QSizeF(236, 96), active);
    QVariantMap arguments;
    arguments.insert(QStringLiteral("bridge"), QVariant::fromValue(static_cast<KDecoration3::DecorationBridge *>(made.bridge.get())));
    if (!theme.isEmpty()) {
        arguments.insert(QStringLiteral("theme"), theme);
    }
    made.decoration.reset(factory->create<KDecoration3::Decoration>(nullptr, QVariantList{arguments}));
    if (!made.decoration) {
        return made;
    }
    // The page gives the decoration an item of its own scene to draw in. Without one,
    // the decorations written in QML (Aurorae) make a renderer of their own, which
    // needs OpenGL.
    if (visualParent) {
        made.decoration->setProperty("visualParent", QVariant::fromValue(visualParent));
    }
    made.settings = std::make_shared<KDecoration3::DecorationSettings>(made.bridge.get());
    made.decoration->setSettings(made.settings);
    made.decoration->create();
    made.decoration->init();
    made.decoration->apply(made.decoration->nextState()->clone());
    return made;
}

// The shadow of the decoration, as the preview of the page draws it: the four
// corners at their size, and the four sides stretched between them, around the
// window by the padding that the shadow asks for.
void paintShadow(QPainter &painter, KDecoration3::Decoration &decoration, const QSizeF &size)
{
    const auto shadow = decoration.shadow();
    if (!shadow) {
        return;
    }
    const QImage image = shadow->shadow();
    if (image.isNull()) {
        return;
    }
    const QRectF outer(-shadow->paddingLeft(), -shadow->paddingTop(), size.width() + shadow->paddingLeft() + shadow->paddingRight(), size.height() + shadow->paddingTop() + shadow->paddingBottom());
    const QRectF tl = shadow->topLeftGeometry();
    const QRectF t = shadow->topGeometry();
    const QRectF tr = shadow->topRightGeometry();
    const QRectF r = shadow->rightGeometry();
    const QRectF br = shadow->bottomRightGeometry();
    const QRectF b = shadow->bottomGeometry();
    const QRectF bl = shadow->bottomLeftGeometry();
    const QRectF l = shadow->leftGeometry();

    const auto tile = [&](const QRectF &target, const QRectF &source) {
        if (target.width() > 0 && target.height() > 0 && !source.isEmpty()) {
            painter.drawImage(target, image, source);
        }
    };
    tile(QRectF(outer.left(), outer.top(), tl.width(), tl.height()), tl);
    tile(QRectF(outer.right() - tr.width(), outer.top(), tr.width(), tr.height()), tr);
    tile(QRectF(outer.left(), outer.bottom() - bl.height(), bl.width(), bl.height()), bl);
    tile(QRectF(outer.right() - br.width(), outer.bottom() - br.height(), br.width(), br.height()), br);
    tile(QRectF(outer.left() + tl.width(), outer.top(), outer.width() - tl.width() - tr.width(), t.height()), t);
    tile(QRectF(outer.left() + bl.width(), outer.bottom() - b.height(), outer.width() - bl.width() - br.width(), b.height()), b);
    tile(QRectF(outer.left(), outer.top() + tl.height(), l.width(), outer.height() - tl.height() - bl.height()), l);
    tile(QRectF(outer.right() - r.width(), outer.top() + tr.height(), r.width(), outer.height() - tr.height() - br.height()), r);
}

// The decorations written in QML (Aurorae): each window is drawn in a Qt Quick
// scene with the software renderer, as an item of that scene, and grabbed.
int drawQmlDecoration(KPluginFactory *factory, const QString &theme, const QString &output)
{
    QQuickWindow::setGraphicsApi(QSGRendererInterface::Software);

    const QPalette palette = schemePalette();
    const qreal margin = 12;
    const qreal titleBar = 22;
    const QSizeF itemSize(kCardWidth - 2 * margin - titleBar, kCardHeight - 2 * margin - titleBar);

    QImage card(kCardWidth, kCardHeight, QImage::Format_ARGB32_Premultiplied);
    card.fill(palette.color(QPalette::Base));
    QPainter painter(&card);

    // An inactive window behind and the active one in front of it, as on the page.
    for (const bool active : {false, true}) {
        QQuickWindow window;
        window.setColor(Qt::transparent);
        window.resize(itemSize.toSize());
        auto *item = new QQuickItem(window.contentItem());
        item->setSize(itemSize);

        PreviewDecoration made = makeDecoration(factory, theme, palette, active, item);
        if (!made.decoration) {
            return 2;
        }
        const QMarginsF borders = made.decoration->borders();
        const QSizeF client(itemSize.width() - borders.left() - borders.right(), itemSize.height() - borders.top() - borders.bottom());
        made.bridge->lastCreatedClient()->setSize(client);

        window.show();
        QCoreApplication::processEvents();
        const QImage drawn = window.grabWindow();
        if (drawn.isNull()) {
            return 3;
        }
        const QPointF origin = active ? QPointF(margin + titleBar, margin + titleBar) : QPointF(margin, margin);
        painter.drawImage(origin, drawn);
        painter.fillRect(QRectF(origin + QPointF(borders.left(), borders.top()), client), palette.color(QPalette::Window));
    }
    painter.end();
    return card.save(output, "PNG") ? 0 : 3;
}

int drawDecoration(const QString &pluginId, const QString &theme, const QString &caption, const QString &output)
{
    const QList<KPluginMetaData> plugins = KPluginMetaData::findPlugins(QStringLiteral("org.kde.kdecoration3"));
    const auto found = std::find_if(plugins.begin(), plugins.end(), [&](const KPluginMetaData &data) { return data.pluginId() == pluginId; });
    if (found == plugins.end()) {
        return 2;
    }
    const auto loaded = KPluginFactory::loadFactory(*found);
    if (!loaded.plugin) {
        return 2;
    }

    // The icon theme in use, for the icon of the menu button: with no desktop
    // platform here, Qt does not know it.
    const QString iconTheme = KSharedConfig::openConfig(QStringLiteral("kdeglobals"))->group(QStringLiteral("Icons")).readEntry("Theme", QString());
    if (!iconTheme.isEmpty()) {
        QIcon::setThemeName(iconTheme);
    }

    // The windows carry the name of the decoration, as on the page.
    QString name = theme;
    for (const QString &prefix : {QStringLiteral("__aurorae__svg__"), QStringLiteral("__aurorae__qml__")}) {
        if (name.startsWith(prefix)) {
            name = name.mid(prefix.size());
        }
    }
    // The name that the list shows, when it is given, is the one that the page shows.
    if (!caption.isEmpty()) {
        name = caption;
    }
    if (!name.isEmpty()) {
        g_caption = name;
    }

    // The engines of Aurorae draw QML: they go through a Qt Quick scene.
    if (pluginId.startsWith(QLatin1String("org.kde.kwin.aurorae"))) {
        return drawQmlDecoration(loaded.plugin, theme, output);
    }

    const QPalette palette = schemePalette();
    PreviewDecoration inactive = makeDecoration(loaded.plugin, theme, palette, false);
    PreviewDecoration active = makeDecoration(loaded.plugin, theme, palette, true);
    if (!inactive.decoration || !active.decoration) {
        return 2;
    }

    // A decoration that draws nothing here (the ones written in QML need a scene
    // of Qt Quick that this helper does not have) is not saved, and the card
    // keeps its icon. It is painted alone on a transparent image to know.
    {
        QImage probe(kCardWidth, kCardHeight, QImage::Format_ARGB32_Premultiplied);
        probe.fill(Qt::transparent);
        QPainter painter(&probe);
        const QMarginsF borders = active.decoration->borders();
        active.bridge->lastCreatedClient()->setSize(QSizeF(200 - borders.left() - borders.right(), 120 - borders.top() - borders.bottom()));
        active.decoration->paint(&painter, QRectF(0, 0, 200, 120));
        painter.end();
        bool drew = false;
        for (int y = 0; y < probe.height() && !drew; ++y) {
            for (int x = 0; x < probe.width(); ++x) {
                if (qAlpha(probe.pixel(x, y)) != 0) {
                    drew = true;
                    break;
                }
            }
        }
        if (!drew) {
            return 3;
        }
    }

    // The page shows two windows, an inactive one behind and the active one in
    // front of it, shifted by the height of the title bar, on the color of the
    // base of the palette (kcms/decoration/ui/Themes.qml).
    const qreal margin = 12;
    const qreal titleBar = active.decoration->titleBar().height();
    const QSizeF itemSize(kCardWidth - 2 * margin - titleBar, kCardHeight - 2 * margin - titleBar);

    QImage image(kCardWidth, kCardHeight, QImage::Format_ARGB32_Premultiplied);
    image.fill(palette.color(QPalette::Base));
    {
        QPainter painter(&image);
        painter.setRenderHint(QPainter::Antialiasing);

        const auto paint = [&](PreviewDecoration &window, const QPointF &origin) {
            const QMarginsF borders = window.decoration->borders();
            window.bridge->lastCreatedClient()->setSize(QSizeF(itemSize.width() - borders.left() - borders.right(), itemSize.height() - borders.top() - borders.bottom()));
            painter.save();
            painter.translate(origin);
            paintShadow(painter, *window.decoration, itemSize);
            window.decoration->paint(&painter, QRectF(QPointF(0, 0), itemSize));
            painter.fillRect(QRectF(QPointF(borders.left(), borders.top()), QSizeF(itemSize.width() - borders.left() - borders.right(), itemSize.height() - borders.top() - borders.bottom())), palette.color(QPalette::Window));
            painter.restore();
        };
        paint(inactive, QPointF(margin, margin));
        paint(active, QPointF(margin + titleBar, margin + titleBar));
    }

    return image.save(output, "PNG") ? 0 : 3;
}

// ── Widgets of Plasma ────────────────────────────────────────────────────────

// Every widget that Plasma has, as Plasma lists them (`listAppletMetaData` of
// libplasma): the packages of QML of the type `Plasma/Applet` and the plugins that
// are compiled (`plasma/applets/`), which do not have a metadata.json to read. It
// prints a JSON list, with the names in the language of the process.
int listWidgets()
{
    QList<KPluginMetaData> all = KPackage::PackageLoader::self()->findPackages(QStringLiteral("Plasma/Applet"), QString());
    const QList<KPluginMetaData> compiled = KPluginMetaData::findPlugins(QStringLiteral("plasma/applets/"), {}, KPluginMetaData::AllowEmptyMetaData);
    for (const KPluginMetaData &plugin : compiled) {
        const bool known = std::any_of(all.begin(), all.end(), [&](const KPluginMetaData &other) { return other.pluginId() == plugin.pluginId(); });
        if (!known) {
            all.append(plugin);
        }
    }

    QJsonArray out;
    for (const KPluginMetaData &widget : std::as_const(all)) {
        QJsonObject item;
        item.insert(QStringLiteral("id"), widget.pluginId());
        item.insert(QStringLiteral("name"), widget.name());
        item.insert(QStringLiteral("description"), widget.description());
        item.insert(QStringLiteral("category"), widget.category());
        item.insert(QStringLiteral("icon"), widget.iconName());
        item.insert(QStringLiteral("formFactors"), QJsonArray::fromStringList(widget.formFactors()));
        // The widgets of the system tray go in the tray, not on a panel.
        const QJsonValue tray = widget.rawData().value(QStringLiteral("X-Plasma-NotificationArea"));
        item.insert(QStringLiteral("tray"), tray.toBool() || tray.toString() == QLatin1String("true"));
        // A containment (the desktop, the folder view) is not a widget to put on a panel.
        item.insert(QStringLiteral("containment"), widget.rawData().contains(QStringLiteral("X-Plasma-ContainmentType")));
        out.append(item);
    }
    fputs(QJsonDocument(out).toJson(QJsonDocument::Compact).constData(), stdout);
    return 0;
}

// ── Things of KWin ──────────────────────────────────────────────────────────

// Prints, as a JSON list, what KWin has of a kind, found as the pages of System
// Settings find it (kcms/common/effectsmodel.cpp, kcms/scripts and kcms/tabbox of KWin):
//   effects  the native plugins (`kwin/effects/plugins`), the packages of the type
//            KWin/Effect and the built-in effects (JSON in `kwin-wayland/builtin-effects`)
//   scripts  the packages of the type KWin/Script
//   tabbox   the packages of the type KWin/WindowSwitcher
int listKwin(const QString &what)
{
    QList<KPluginMetaData> all;
    const auto addPackages = [&all](const QString &type, const QString &root) {
        const QList<KPluginMetaData> found = KPackage::PackageLoader::self()->listPackages(type, root);
        for (const KPluginMetaData &package : found) {
            const bool known = std::any_of(all.begin(), all.end(), [&](const KPluginMetaData &other) { return other.pluginId() == package.pluginId(); });
            if (!known) {
                all.append(package);
            }
        }
    };

    if (what == QLatin1String("effects")) {
        all = KPluginMetaData::findPlugins(QStringLiteral("kwin/effects/plugins"));
        addPackages(QStringLiteral("KWin/Effect"), QStringLiteral("kwin-wayland/effects"));
        addPackages(QStringLiteral("KWin/Effect"), QStringLiteral("kwin/effects"));
        const QStringList dirs = QStandardPaths::locateAll(QStandardPaths::GenericDataLocation, QStringLiteral("kwin-wayland/builtin-effects"), QStandardPaths::LocateDirectory);
        for (const QString &dir : dirs) {
            const QStringList files = QDir(dir).entryList({QStringLiteral("*.json")}, QDir::Files);
            for (const QString &file : files) {
                const KPluginMetaData builtin = KPluginMetaData::fromJsonFile(dir + QLatin1Char('/') + file);
                if (builtin.isValid()) {
                    all.append(builtin);
                }
            }
        }
    } else if (what == QLatin1String("scripts")) {
        addPackages(QStringLiteral("KWin/Script"), QStringLiteral("kwin/scripts"));
    } else if (what == QLatin1String("tabbox")) {
        addPackages(QStringLiteral("KWin/WindowSwitcher"), QStringLiteral("kwin-wayland/tabbox"));
        addPackages(QStringLiteral("KWin/WindowSwitcher"), QStringLiteral("kwin/tabbox"));
    } else {
        return 1;
    }

    QJsonArray out;
    for (const KPluginMetaData &item : std::as_const(all)) {
        const QJsonValue internal = item.rawData().value(QStringLiteral("X-KWin-Internal"));
        if (internal.toBool() || internal.toString() == QLatin1String("true")) {
            continue;
        }
        QJsonObject object;
        object.insert(QStringLiteral("id"), item.pluginId());
        object.insert(QStringLiteral("name"), item.name());
        object.insert(QStringLiteral("description"), item.description());
        object.insert(QStringLiteral("category"), item.category());
        object.insert(QStringLiteral("icon"), item.iconName());
        object.insert(QStringLiteral("enabledByDefault"), item.isEnabledByDefault());
        out.append(object);
    }
    fputs(QJsonDocument(out).toJson(QJsonDocument::Compact).constData(), stdout);
    return 0;
}

} // namespace

int main(int argc, char *argv[])
{
    // The pictures are drawn in memory: no window and no display are needed.
    qputenv("QT_QPA_PLATFORM", "offscreen");

    // The names of the widgets come in the language of the program.
    if (const QByteArray lang = qgetenv("SOPLOS_PREVIEW_LANG"); !lang.isEmpty()) {
        qputenv("LANGUAGE", lang);
    }

    QApplication app(argc, argv);
    const QStringList args = app.arguments();

    if (args.size() == 4 && args.at(1) == QLatin1String("style")) {
        return drawStyle(args.at(2), args.at(3));
    }
    if (args.size() == 4 && args.at(1) == QLatin1String("plasma")) {
        return drawPlasma(args.at(2), args.at(3));
    }
    if (args.size() == 3 && args.at(1) == QLatin1String("kwin")) {
        return listKwin(args.at(2));
    }
    if (args.size() == 2 && args.at(1) == QLatin1String("widgets")) {
        return listWidgets();
    }
    if (args.size() == 6 && args.at(1) == QLatin1String("decoration")) {
        return drawDecoration(args.at(2), args.at(3), args.at(4), args.at(5));
    }
    return 1;
}

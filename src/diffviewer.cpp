#include "diffviewer.h"

#include <QPainter>
#include <QRegularExpression>
#include <QFontDatabase>
#include <QScrollBar>
#include <QTextBlock>

// --- LineNumberArea ---

LineNumberArea::LineNumberArea(DiffViewer *editor)
    : QWidget(editor), m_editor(editor)
{
}

QSize LineNumberArea::sizeHint() const
{
    return QSize(m_editor->lineNumberAreaWidth(), 0);
}

void LineNumberArea::paintEvent(QPaintEvent *event)
{
    m_editor->paintLineNumbers(event);
}

// --- DiffHighlighter ---

DiffHighlighter::DiffHighlighter(QTextDocument *parent)
    : QSyntaxHighlighter(parent)
{
    m_additionFormat.setBackground(QColor(0, 200, 0, 25));
    m_additionFormat.setForeground(QColor(40, 170, 40));
    m_deletionFormat.setBackground(QColor(200, 0, 0, 25));
    m_deletionFormat.setForeground(QColor(200, 60, 60));
    m_hunkFormat.setForeground(QColor(0, 120, 255));
    m_hunkFormat.setFontWeight(QFont::Bold);
}

void DiffHighlighter::highlightBlock(const QString &text)
{
    if (text.startsWith('+'))
        setFormat(0, text.length(), m_additionFormat);
    else if (text.startsWith('-'))
        setFormat(0, text.length(), m_deletionFormat);
    else if (text.startsWith("@@"))
        setFormat(0, text.length(), m_hunkFormat);
}

// --- DiffViewer ---

DiffViewer::DiffViewer(QWidget *parent)
    : QPlainTextEdit(parent)
{
    m_lineNumberArea = new LineNumberArea(this);
    m_highlighter = new DiffHighlighter(document());

    setReadOnly(true);
    setLineWrapMode(QPlainTextEdit::NoWrap);
    setFont(QFontDatabase::systemFont(QFontDatabase::FixedFont));

    connect(this, &QPlainTextEdit::blockCountChanged,
            this, &DiffViewer::updateLineNumberAreaWidth);
    connect(this, &QPlainTextEdit::updateRequest,
            this, &DiffViewer::updateLineNumberArea);

    updateLineNumberAreaWidth(0);
}

void DiffViewer::setDiff(const QString &rawDiff)
{
    static const QRegularExpression headerRe(
        R"(^(---|\+\+\+|diff --git|\\ ))");

    QStringList result;
    const QStringList lines = rawDiff.split('\n');
    for (const QString &line : lines) {
        if (headerRe.match(line).hasMatch())
            continue;
        result.append(line);
    }

    setPlainText(result.join('\n'));
    verticalScrollBar()->setValue(0);
}

void DiffViewer::clear()
{
    setPlainText(QString());
}

void DiffViewer::resizeEvent(QResizeEvent *event)
{
    QPlainTextEdit::resizeEvent(event);
    const QRect cr = contentsRect();
    m_lineNumberArea->setGeometry(
        cr.left(), cr.top(), lineNumberAreaWidth(), cr.height());
}

void DiffViewer::updateLineNumberAreaWidth(int)
{
    setViewportMargins(lineNumberAreaWidth(), 0, 0, 0);
}

void DiffViewer::updateLineNumberArea(const QRect &rect, int dy)
{
    if (dy)
        m_lineNumberArea->scroll(0, dy);
    else
        m_lineNumberArea->update(0, rect.y(),
            m_lineNumberArea->width(), rect.height());

    if (rect.contains(viewport()->rect()))
        updateLineNumberAreaWidth(0);
}

int DiffViewer::lineNumberAreaWidth() const
{
    int digits = 1;
    int max = qMax(1, blockCount());
    while (max >= 10) {
        max /= 10;
        ++digits;
    }
    return 8 + fontMetrics().horizontalAdvance(QLatin1Char('9')) * digits;
}

void DiffViewer::paintLineNumbers(QPaintEvent *event)
{
    QPainter painter(m_lineNumberArea);
    painter.fillRect(event->rect(), palette().window().color());

    const int areaWidth = m_lineNumberArea->width();
    QTextBlock block = firstVisibleBlock();
    int blockNumber = block.blockNumber();
    qreal top = blockBoundingGeometry(block).translated(contentOffset()).top();
    qreal bottom = top + blockBoundingRect(block).height();

    while (block.isValid() && top <= event->rect().bottom()) {
        if (block.isVisible() && bottom >= event->rect().top()) {
            painter.setPen(palette().color(QPalette::Disabled, QPalette::Text));
            painter.drawText(0, qRound(top),
                areaWidth - 4, fontMetrics().height(),
                Qt::AlignRight | Qt::AlignVCenter,
                QString::number(blockNumber + 1));
        }

        block = block.next();
        top = bottom;
        bottom = top + blockBoundingRect(block).height();
        ++blockNumber;
    }
}

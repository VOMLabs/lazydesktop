#ifndef DIFFVIEWER_H
#define DIFFVIEWER_H

#include <QPlainTextEdit>
#include <QSyntaxHighlighter>

class DiffViewer;

class LineNumberArea : public QWidget
{
public:
    explicit LineNumberArea(DiffViewer *editor);
    QSize sizeHint() const override;

protected:
    void paintEvent(QPaintEvent *event) override;

private:
    DiffViewer *m_editor;
};

class DiffHighlighter : public QSyntaxHighlighter
{
    Q_OBJECT
public:
    explicit DiffHighlighter(QTextDocument *parent);

protected:
    void highlightBlock(const QString &text) override;

private:
    QTextCharFormat m_additionFormat;
    QTextCharFormat m_deletionFormat;
    QTextCharFormat m_hunkFormat;
};

class DiffViewer : public QPlainTextEdit
{
    Q_OBJECT
    friend class LineNumberArea;

public:
    explicit DiffViewer(QWidget *parent = nullptr);
    void setDiff(const QString &rawDiff);
    void clear();

protected:
    void resizeEvent(QResizeEvent *event) override;

private slots:
    void updateLineNumberAreaWidth(int newBlockCount);
    void updateLineNumberArea(const QRect &rect, int dy);

private:
    int lineNumberAreaWidth() const;
    void paintLineNumbers(QPaintEvent *event);

    QWidget *m_lineNumberArea = nullptr;
    DiffHighlighter *m_highlighter = nullptr;
};

#endif // DIFFVIEWER_H

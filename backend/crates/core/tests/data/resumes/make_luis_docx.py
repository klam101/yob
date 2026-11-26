import sys, zipfile
from xml.sax.saxutils import escape
out = sys.argv[1]
W = 'xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"'
def p(text, bullet=False, bold=False, tab=None):
    ppr = '<w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr>' if bullet else ''
    rpr = '<w:rPr><w:b/></w:rPr>' if bold else ''
    runs = f'<w:r>{rpr}<w:t xml:space="preserve">{escape(text)}</w:t></w:r>'
    if tab:
        runs += f'<w:r><w:tab/><w:t>{escape(tab)}</w:t></w:r>'
    return f'<w:p>{ppr}{runs}</w:p>'
body = [
    p('Luis Fernández', bold=True),
    p('Portland, OR 97205 | luis.fernandez@example.com | 503-555-0181 | github.com/lfernandez-example | ')[:-6]
    + '<w:hyperlink r:id="rId2"><w:r><w:t>Portfolio</w:t></w:r></w:hyperlink></w:p>',
    p('WORK EXPERIENCE', bold=True),
    p('Product Designer | Margie’s Travel', tab='Feb 2023 – Present'),
    p('Redesigned the booking flow & cut drop-off at checkout by 18%.', bullet=True),
    p('Ran 25 usability sessions with travelers over 65.', bullet=True),
    p('UX Design Intern | Wingtip Toys', tab='Jun 2022 – Aug 2022'),
    p('Prototyped a gift registry in Figma that shipped the next quarter.', bullet=True),
    p('EDUCATION', bold=True),
    p('Portland State University', tab='2018 – 2022'),
    p('B.F.A. Graphic Design'),
    p('SKILLS', bold=True),
    p('Figma, Sketch, User Research, Prototyping, HTML, CSS'),
]
doc = f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document {W}><w:body>{"".join(body)}</w:body></w:document>'
numbering = f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:numbering {W}><w:abstractNum w:abstractNumId="0"><w:lvl w:ilvl="0"><w:numFmt w:val="bullet"/><w:lvlText w:val="•"/></w:lvl></w:abstractNum><w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num></w:numbering>'
ct = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/numbering.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml"/></Types>'
rels = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>'
drels = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering" Target="numbering.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://luis-fernandez.example.design/" TargetMode="External"/></Relationships>'
with zipfile.ZipFile(out, 'w', zipfile.ZIP_DEFLATED) as z:
    z.writestr('[Content_Types].xml', ct)
    z.writestr('_rels/.rels', rels)
    z.writestr('word/document.xml', doc)
    z.writestr('word/_rels/document.xml.rels', drels)
    z.writestr('word/numbering.xml', numbering)

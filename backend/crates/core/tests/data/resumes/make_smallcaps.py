"""Writes tests/data/resumes/smallcaps.txt: a fictional resume as text
extraction returns it from a LaTeX template with small caps, Font Awesome
icons and U+2011 hyphens."""
import sys

H = "‑"  # non-breaking hyphen
PHONE, MAIL, GH, LI = "", "", "", ""
text = f"""
Morgan Ellis

{PHONE} (555) 555{H}0147 | {MAIL} morgan.ellis@example.com | portfolio | {GH} mellis{H}example | {LI} morgan{H}ellis{H}example

Education

Lakeside State University Class of 2025
BACHELOR OF SCiENCE iN iNFORMATiON SYSTEMS, MiNOR STATiSTiCS GPA: 3.71/4.0
• Coursework {H} Databases, Data Mining, Systems Analysis

Experience

Northwind Traders Columbus, OH
DATA ANALYST iNTERN May 2024 {H} Aug 2024
• Built a self{H}service sales dashboard in Power BI used by 40 regional managers, replacing a weekly spread{H}
sheet export and cutting report requests by 60%
• Automated data quality checks in Python that caught 1,200 duplicate customer records before the quarterly deliv{H}
ery to finance

Contoso Analytics {H} Student Consulting Group Columbus, OH
BUSiNESS iNTELLiGENCE ASSiSTANT Sep 2023 {H} Apr 2024
• Modeled churn for a local gym chain with logistic regression, presenting findings to the owners

Projects

Tidepool  {GH} GitHub
COMMUNiTY WATER QUALiTY MAP
• Mapped 300 public water test results on an interactive map with React and Leaflet

Shelfie  {GH} GitHub
BOOK TRADiNG APP
• Built a book swap app with Flask and SQLite for a campus club of 80 members

Skills

Languages Python, SQL, R, JavaScript
Technologies Power BI, Tableau, PostgreSQL, Git, Excel
Data Visualization Tools Leaflet, D3.js, Matplotlib, Seaborn
"""
open(sys.argv[1], "w", encoding="utf-8", newline="\n").write(text.lstrip("\n"))

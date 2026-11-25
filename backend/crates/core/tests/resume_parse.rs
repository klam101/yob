//! The rule-based resume parser against fictional resumes in different
//! layouts (`tests/data/resumes/`). Run with `SHOW_PARSE=1` and
//! `--nocapture` to print what each one parses into.

use yob_core::profile::{
    Experience, LinkKind,
    parse::{Parsed, parse},
};

fn load(name: &str) -> Parsed {
    let path = format!("{}/tests/data/resumes/{name}", env!("CARGO_MANIFEST_DIR"));
    let parsed = parse(&std::fs::read_to_string(path).unwrap(), &[]);
    if std::env::var("SHOW_PARSE").is_ok() {
        eprintln!(
            "==== {name}\n{:#?}\nunsorted: {:#?}",
            parsed.profile, parsed.unsorted
        );
    }
    parsed
}

fn job(e: &Experience) -> (&str, &str, &str, Option<&str>, Option<&str>) {
    (
        &e.title,
        &e.company,
        &e.location,
        e.start.as_deref(),
        e.end.as_deref(),
    )
}

#[test]
fn classic_single_column() {
    let Parsed {
        profile: p,
        unsorted,
        ..
    } = load("classic.txt");
    let me = &p.personal;
    assert_eq!(
        (me.first_name.as_str(), me.last_name.as_str()),
        ("Jordan", "A. Rivera")
    );
    assert_eq!(me.email, "jordan.rivera@example.com");
    assert_eq!(me.phone, "(217) 555-0142");
    assert_eq!(
        (
            me.street.as_str(),
            me.city.as_str(),
            me.region.as_str(),
            me.postal_code.as_str()
        ),
        ("1420 Maple Avenue", "Springfield", "IL", "62704")
    );
    assert!(me.summary.starts_with("Backend engineer"));
    assert_eq!(p.links.len(), 2);
    assert_eq!(p.links[0].kind, LinkKind::Linkedin);
    assert_eq!(p.links[1].url, "https://github.com/jrivera-example");

    assert_eq!(p.experiences.len(), 2);
    assert_eq!(
        job(&p.experiences[0]),
        (
            "Senior Software Engineer",
            "Northwind Logistics",
            "Chicago, IL",
            Some("2022-03"),
            None
        )
    );
    // The wrapped line is joined back onto its bullet.
    assert_eq!(p.experiences[0].bullets.len(), 2);
    assert!(p.experiences[0].bullets[0].ends_with("from 900 ms to 180 ms."));
    assert_eq!(
        job(&p.experiences[1]),
        (
            "Software Engineer",
            "Contoso Payments",
            "Remote",
            Some("2019-06"),
            Some("2022-02")
        )
    );

    let edu = &p.educations[0];
    assert_eq!(edu.school, "University of Illinois Urbana-Champaign");
    assert_eq!(
        (edu.degree.as_str(), edu.field_of_study.as_str()),
        ("Bachelor of Science", "Computer Science")
    );
    assert_eq!(edu.gpa, "3.7/4.0");
    assert_eq!(
        (edu.start.as_deref(), edu.end.as_deref()),
        (Some("2015-08"), Some("2019-05"))
    );

    assert_eq!(p.skills.len(), 9);
    assert_eq!(
        (p.skills[0].name.as_str(), p.skills[0].category.as_str()),
        ("Go", "Languages")
    );
    assert_eq!(p.skills[8].category, "Tools");
    assert!(unsorted.is_empty(), "{unsorted:?}");
}

#[test]
fn pipe_separated_lines() {
    let Parsed {
        profile: p,
        unsorted,
        ..
    } = load("pipes.txt");
    assert_eq!(p.personal.first_name, "Priya");
    assert_eq!(
        (p.personal.city.as_str(), p.personal.postal_code.as_str()),
        ("Austin", "78701")
    );
    assert_eq!(p.links[0].kind, LinkKind::Portfolio);
    assert_eq!(
        job(&p.experiences[0]),
        (
            "Software Engineer",
            "Fabrikam Robotics",
            "Austin, TX",
            Some("2022-06"),
            None
        )
    );
    let intern = &p.experiences[1];
    assert_eq!(intern.company, "Tailspin Toys");
    assert_eq!(intern.employment_type, "Internship");
    assert_eq!(intern.start.as_deref(), Some("2021-06"));
    let edu = &p.educations[0];
    assert_eq!(edu.school, "The University of Texas at Austin");
    assert_eq!(
        (edu.degree.as_str(), edu.field_of_study.as_str()),
        ("B.S.", "Electrical and Computer Engineering")
    );
    assert_eq!(p.skills.len(), 6);
    assert!(unsorted.is_empty(), "{unsorted:?}");
}

#[test]
fn one_field_per_line_without_bullet_markers() {
    let Parsed {
        profile: p,
        unsorted,
        ..
    } = load("multiline.txt");
    assert_eq!(p.personal.headline, "Data Analyst");
    assert_eq!(p.experiences.len(), 2);
    assert_eq!(
        job(&p.experiences[0]),
        (
            "Data Analyst",
            "Woodgrove Bank",
            "Atlanta, GA",
            Some("2021-01"),
            None
        )
    );
    assert_eq!(p.experiences[0].bullets.len(), 1);
    assert_eq!(
        job(&p.experiences[1]),
        (
            "Junior Analyst",
            "Adventure Works",
            "",
            Some("2019-06"),
            Some("2020-12")
        )
    );
    assert_eq!(p.experiences[1].bullets.len(), 1);
    assert_eq!(p.educations[0].degree, "Master of Science");
    assert_eq!(p.educations[0].field_of_study, "Analytics");
    assert!(unsorted.is_empty(), "{unsorted:?}");
}

#[test]
fn student_with_projects_and_coursework() {
    let Parsed {
        profile: p,
        unsorted,
        ..
    } = load("student.txt");
    let edu = &p.educations[0];
    assert_eq!(
        (edu.school.as_str(), edu.location.as_str()),
        ("Clemson University", "Clemson, SC")
    );
    assert_eq!(
        (edu.degree.as_str(), edu.field_of_study.as_str()),
        ("B.S.", "Computer Science")
    );
    assert_eq!(edu.gpa, "3.52/4.00");
    assert_eq!(edu.end.as_deref(), Some("2026-05"));
    assert_eq!(edu.minor, "Mathematics");
    assert!(edu.details.contains("Relevant Coursework: Data Structures"));

    assert_eq!(p.projects.len(), 2);
    assert_eq!(p.projects[0].name, "Campus Ride Share");
    assert_eq!(p.projects[0].skills, ["React", "Node.js", "PostgreSQL"]);
    assert_eq!(p.projects[0].bullets.len(), 2);
    // "Technologies: ..." moves from the bullets to the project's skills.
    assert_eq!(p.projects[1].skills, ["Python", "Flask", "SQLite"]);
    assert_eq!(p.projects[1].bullets.len(), 1);

    assert_eq!(
        job(&p.experiences[0]),
        (
            "Teaching Assistant, School of Computing",
            "Clemson University",
            "Clemson, SC",
            Some("2024-08"),
            None
        )
    );
    assert_eq!(
        p.skills
            .iter()
            .filter(|s| s.category == "Frameworks & Tools")
            .count(),
        4
    );
    // Awards aren't stored, so they're shown for review.
    assert_eq!(unsorted.len(), 1);
    assert!(unsorted[0].contains("Dean's List"));
}

#[test]
fn caps_and_letter_spaced_headings() {
    let Parsed {
        profile: p,
        unsorted,
        ..
    } = load("caps.txt");
    assert_eq!(
        (
            p.personal.first_name.as_str(),
            p.personal.last_name.as_str()
        ),
        ("Dana", "K. Whitfield")
    );
    assert_eq!(p.personal.phone, "+1 (615) 555-0110");
    assert_eq!(p.experiences.len(), 2);
    assert_eq!(
        job(&p.experiences[0]),
        (
            "QA Engineer",
            "Litware Health",
            "Nashville, TN",
            Some("2020-01"),
            Some("2023-12")
        )
    );
    assert_eq!(p.experiences[0].bullets.len(), 2);
    assert_eq!(p.experiences[1].title, "Quality Assurance Tester");
    assert_eq!(p.educations[0].school, "Middle Tennessee State University");
    assert_eq!(p.educations[0].degree, "B.A.");
    assert_eq!(unsorted.len(), 1);
    assert!(unsorted[0].contains("ISTQB"));
}

fn file(name: &str) -> Parsed {
    use yob_core::profile::resume::{detect, extract};
    let path = format!("{}/tests/data/resumes/{name}", env!("CARGO_MANIFEST_DIR"));
    let bytes = std::fs::read(path).unwrap();
    let kind = detect(&bytes).expect("recognized file type");
    let x = extract(kind, &bytes).unwrap();
    if std::env::var("SHOW_PARSE").is_ok() {
        eprintln!("==== {name} text\n{}\nlinks: {:?}", x.text, x.links);
    }
    let parsed = parse(&x.text, &x.links);
    if std::env::var("SHOW_PARSE").is_ok() {
        eprintln!(
            "==== {name}\n{:#?}\nunsorted: {:#?}",
            parsed.profile, parsed.unsorted
        );
    }
    parsed
}

/// A PDF printed by Edge from `avery.html`: text extraction loses the
/// two-sided rows, so this is the realistic case.
#[test]
fn pdf_file() {
    let Parsed { profile: p, .. } = file("avery.pdf");
    assert_eq!(p.personal.first_name, "Avery");
    assert_eq!(p.personal.email, "avery.thompson@example.com");
    assert_eq!(p.personal.city, "Denver");
    // The link wrapped onto a second line is joined back up.
    assert_eq!(
        p.links[0].url,
        "https://linkedin.com/in/avery-thompson-example"
    );
    assert_eq!(p.experiences.len(), 2);
    // "Fourth Coffee Denver, CO": two columns run together in the PDF text.
    assert_eq!(p.experiences[0].company, "Fourth Coffee");
    assert_eq!(p.experiences[0].location, "Denver, CO");
    assert_eq!(p.experiences[0].title, "Site Reliability Engineer");
    assert_eq!(p.experiences[0].start.as_deref(), Some("2021-04"));
    assert_eq!(p.experiences[0].end, None);
    assert_eq!(p.experiences[0].bullets.len(), 2);
    assert_eq!(p.experiences[1].title, "Systems Administrator");
    assert_eq!(p.educations[0].school, "Colorado State University");
    assert_eq!(p.educations[0].location, "Fort Collins, CO");
    assert_eq!(
        p.educations[0].field_of_study,
        "Computer Information Systems"
    );
    assert!(
        p.skills
            .iter()
            .any(|s| s.name == "Terraform" && s.category == "Infrastructure")
    );
}

/// A DOCX whose bullets are Word list paragraphs (built by
/// `make_luis_docx.py`).
#[test]
fn docx_file() {
    let Parsed { profile: p, .. } = file("luis.docx");
    assert_eq!(
        (
            p.personal.first_name.as_str(),
            p.personal.last_name.as_str()
        ),
        ("Luis", "Fernández")
    );
    assert_eq!(p.personal.region, "OR");
    assert_eq!(p.experiences.len(), 2);
    assert_eq!(p.experiences[0].title, "Product Designer");
    assert_eq!(p.experiences[0].company, "Margie’s Travel");
    // The escaped "&" survives the XML.
    assert!(p.experiences[0].bullets[0].contains("flow & cut"));
    assert_eq!(p.experiences[1].employment_type, "Internship");
    assert_eq!(p.educations[0].degree, "B.F.A.");
    assert_eq!(p.educations[0].field_of_study, "Graphic Design");
    assert_eq!(p.skills.len(), 6);
    // "Portfolio" is a hyperlink; its address is only in the DOCX's links.
    assert_eq!(
        p.links
            .iter()
            .map(|l| (l.kind, l.url.as_str()))
            .collect::<Vec<_>>(),
        [
            (LinkKind::Github, "https://github.com/lfernandez-example"),
            (
                LinkKind::Portfolio,
                "https://luis-fernandez.example.design/"
            ),
        ]
    );
}

/// A PDF printed by Edge from `links.html`, whose links show only labels:
/// the addresses come from the PDF's link annotations.
#[test]
fn pdf_link_annotations() {
    let Parsed {
        profile: p,
        unsorted,
        ..
    } = file("links.pdf");
    assert_eq!(p.personal.email, "riley.chen@example.com");
    assert_eq!(
        p.personal.headline, "",
        "a shown user name isn't a headline"
    );
    assert_eq!(
        p.links
            .iter()
            .map(|l| (l.kind, l.url.as_str()))
            .collect::<Vec<_>>(),
        [
            (LinkKind::Portfolio, "https://rileychen.example.dev/"),
            (LinkKind::Github, "https://github.com/rchen-example"),
            (
                LinkKind::Linkedin,
                "https://www.linkedin.com/in/riley-chen-example"
            ),
        ]
    );
    let projects: Vec<(&str, &str)> = p
        .projects
        .iter()
        .map(|x| (x.name.as_str(), x.url.as_str()))
        .collect();
    assert_eq!(
        projects,
        [
            ("Lumen", "https://github.com/rchen-example/lumen"),
            ("Orbit", "https://orbit-demo.example.app/"),
        ]
    );
    assert_eq!(p.experiences[0].bullets.len(), 2);
    assert_eq!(p.educations[0].minor, "Mathematics");
    assert!(unsorted.is_empty(), "{unsorted:?}");
}

/// A LaTeX-style resume as PDF text extraction returns it: small caps with
/// a lowercase "i", U+2011 hyphens, Font Awesome icons, words hyphenated
/// across lines, a minor, skill labels without colons, and project link
/// labels whose addresses are only in the file's links.
#[test]
fn latex_small_caps_and_links() {
    let path = format!(
        "{}/tests/data/resumes/smallcaps.txt",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(path).unwrap();
    // In reading order, as `resume::extract` returns them from a PDF.
    let links: Vec<String> = [
        "tel:+15555550147",
        "mailto:morgan.ellis@example.com",
        "https://morgan-ellis.example.dev/",
        "https://github.com/mellis-example",
        "https://www.linkedin.com/in/morgan-ellis-example/",
        "https://github.com/mellis-example/shelfie",
        "https://tidepool-map.example.org",
    ]
    .map(String::from)
    .to_vec();
    let Parsed {
        profile: p,
        unsorted,
        ..
    } = parse(&text, &links);
    if std::env::var("SHOW_PARSE").is_ok() {
        eprintln!("{p:#?}\nunsorted: {unsorted:#?}");
    }
    let me = &p.personal;
    assert_eq!(
        (me.first_name.as_str(), me.last_name.as_str()),
        ("Morgan", "Ellis")
    );
    assert_eq!(me.phone, "(555) 555-0147");
    assert_eq!(me.headline, "", "a link label isn't a headline");
    let link = |k: LinkKind| p.links.iter().find(|l| l.kind == k).map(|l| l.url.as_str());
    assert_eq!(
        link(LinkKind::Github),
        Some("https://github.com/mellis-example")
    );
    assert_eq!(
        link(LinkKind::Linkedin),
        Some("https://www.linkedin.com/in/morgan-ellis-example/")
    );
    assert_eq!(
        link(LinkKind::Portfolio),
        Some("https://morgan-ellis.example.dev/")
    );

    let edu = &p.educations[0];
    assert_eq!(edu.school, "Lakeside State University");
    assert_eq!(
        (
            edu.degree.as_str(),
            edu.field_of_study.as_str(),
            edu.minor.as_str()
        ),
        ("Bachelor of Science", "Information Systems", "Statistics")
    );
    assert_eq!(edu.gpa, "3.71/4.0");
    assert_eq!(edu.end.as_deref(), Some("2025-05"), "class of 2025");
    assert_eq!(
        edu.details,
        "Coursework: Databases, Data Mining, Systems Analysis"
    );

    assert_eq!(
        p.experiences.iter().map(job).collect::<Vec<_>>(),
        [
            (
                "Data Analyst Intern",
                "Northwind Traders",
                "Columbus, OH",
                Some("2024-05"),
                Some("2024-08")
            ),
            (
                "Business Intelligence Assistant",
                "Contoso Analytics - Student Consulting Group",
                "Columbus, OH",
                Some("2023-09"),
                Some("2024-04")
            ),
        ]
    );
    assert_eq!(p.experiences[0].employment_type, "Internship");
    let b = &p.experiences[0].bullets;
    assert!(b[0].contains("self-service") && b[0].contains("weekly spreadsheet export"));
    assert!(b[1].ends_with("quarterly delivery to finance"));

    let projects: Vec<(&str, &str, &str)> = p
        .projects
        .iter()
        .map(|x| (x.name.as_str(), x.summary.as_str(), x.url.as_str()))
        .collect();
    assert_eq!(
        projects,
        [
            // No name match: the leftover link, in order.
            (
                "Tidepool",
                "Community Water Quality Map",
                "https://tidepool-map.example.org"
            ),
            // By name.
            (
                "Shelfie",
                "Book Trading App",
                "https://github.com/mellis-example/shelfie"
            ),
        ]
    );

    let skill = |name: &str| {
        p.skills
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.category.as_str())
    };
    assert_eq!(skill("Python"), Some("Languages"));
    assert_eq!(skill("Power BI"), Some("Technologies"));
    assert_eq!(skill("Leaflet"), Some("Data Visualization Tools"));
    assert!(unsorted.is_empty(), "{unsorted:?}");
}

/// What the parser changed is recorded by field, with undo where the file's
/// own text is worth putting back.
#[test]
fn recorded_changes() {
    use yob_core::profile::resume_content::ChangeKind;
    let path = format!(
        "{}/tests/data/resumes/smallcaps.txt",
        env!("CARGO_MANIFEST_DIR")
    );
    let parsed = parse(&std::fs::read_to_string(path).unwrap(), &[]);
    let find = |field: &str| {
        parsed
            .changes
            .iter()
            .find(|c| c.field == field)
            .unwrap_or_else(|| panic!("no change for {field}: {:#?}", parsed.changes))
    };
    let degree = find("educations.0.degree");
    assert_eq!(degree.kind, ChangeKind::DegreeSplit);
    assert_eq!(
        degree.before,
        "BACHELOR OF SCIENCE IN INFORMATION SYSTEMS, MINOR STATISTICS"
    );
    // Undo puts the whole line back and clears what was split out of it.
    let undo: Vec<(&str, &str)> = degree
        .undo
        .iter()
        .map(|u| (u.field.as_str(), u.value.as_str().unwrap_or("(not text)")))
        .collect();
    assert_eq!(
        undo,
        [
            ("educations.0.degree", degree.before.as_str()),
            ("educations.0.field_of_study", ""),
            ("educations.0.minor", "")
        ]
    );
    // As a resume's content, the change points at the first row, and undo
    // puts the original line back as the only row.
    let content = yob_core::profile::resume_content::from_parsed(&parsed);
    let split = content
        .changes
        .iter()
        .find(|c| c.kind == ChangeKind::DegreeSplit)
        .unwrap();
    assert_eq!(split.field, "educations.0.degrees.0.degree");
    assert_eq!(split.undo[0].field, "educations.0.degrees");
    assert_eq!(
        split.undo[0].value,
        serde_json::json!([{ "degree": degree.before, "field": "" }])
    );
    let rows: Vec<(&str, &str)> = content.educations[0]
        .degrees
        .iter()
        .map(|d| (d.degree.as_str(), d.field.as_str()))
        .collect();
    assert_eq!(
        rows,
        [
            ("Bachelor of Science", "Information Systems"),
            ("Minor", "Statistics")
        ]
    );
    assert_eq!(find("experiences.0.title").kind, ChangeKind::Capitals);
    assert_eq!(find("experiences.0.title").before, "DATA ANALYST INTERN");
    assert_eq!(find("projects.0.name").kind, ChangeKind::LinkLabel);
    assert_eq!(find("educations.0.end").kind, ChangeKind::ClassOfMay);
    assert_eq!(find("header.links.0.url").kind, ChangeKind::IconLink);
    // A word hyphenated across lines is matched to its own bullet only.
    let joined: Vec<&str> = parsed
        .changes
        .iter()
        .filter(|c| c.kind == ChangeKind::JoinedWord)
        .map(|c| c.field.as_str())
        .collect();
    assert_eq!(
        joined,
        ["experiences.0.bullets.0", "experiences.0.bullets.1"]
    );
    // Small caps inside the degree line are covered by the split, not listed twice.
    assert!(
        !parsed
            .changes
            .iter()
            .any(|c| c.kind == ChangeKind::SmallCaps)
    );
    // Section order follows the file.
    use yob_core::profile::resume_content::SectionKind as S;
    assert_eq!(
        parsed.sections,
        [S::Education, S::Experience, S::Projects, S::Skills]
    );
}

#[test]
fn links_from_the_file_are_recorded() {
    use yob_core::profile::resume_content::ChangeKind;
    let Parsed { changes, .. } = file("links.pdf");
    let from_file: Vec<&str> = changes
        .iter()
        .filter(|c| c.kind == ChangeKind::LinkFromFile)
        .map(|c| c.field.as_str())
        .collect();
    assert_eq!(
        from_file,
        [
            "projects.0.url",
            "projects.1.url",
            "header.links.0.url",
            "header.links.1.url",
            "header.links.2.url"
        ]
    );
}

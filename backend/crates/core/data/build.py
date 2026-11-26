"""Builds backend/crates/core/data/us_places.csv and us_schools.csv from the
Census Bureau's city population estimates (sub-est2023.csv) and the NCES
IPEDS institution directory (HD2023.csv). Both are US government works,
public domain."""
import csv, re, sys

src, out = sys.argv[1], sys.argv[2]

STATES = {
    "Alabama": "AL", "Alaska": "AK", "Arizona": "AZ", "Arkansas": "AR", "California": "CA",
    "Colorado": "CO", "Connecticut": "CT", "Delaware": "DE", "District of Columbia": "DC",
    "Florida": "FL", "Georgia": "GA", "Hawaii": "HI", "Idaho": "ID", "Illinois": "IL",
    "Indiana": "IN", "Iowa": "IA", "Kansas": "KS", "Kentucky": "KY", "Louisiana": "LA",
    "Maine": "ME", "Maryland": "MD", "Massachusetts": "MA", "Michigan": "MI", "Minnesota": "MN",
    "Mississippi": "MS", "Missouri": "MO", "Montana": "MT", "Nebraska": "NE", "Nevada": "NV",
    "New Hampshire": "NH", "New Jersey": "NJ", "New Mexico": "NM", "New York": "NY",
    "North Carolina": "NC", "North Dakota": "ND", "Ohio": "OH", "Oklahoma": "OK", "Oregon": "OR",
    "Pennsylvania": "PA", "Rhode Island": "RI", "South Carolina": "SC", "South Dakota": "SD",
    "Tennessee": "TN", "Texas": "TX", "Utah": "UT", "Vermont": "VT", "Virginia": "VA",
    "Washington": "WA", "West Virginia": "WV", "Wisconsin": "WI", "Wyoming": "WY",
}

SUFFIX = re.compile(
    r"\s+(?:(?:consolidated|metropolitan|unified) government(?: \(balance\))?|city and borough|"
    r"city \(balance\)|\(balance\)|city|City|town|village|borough|township|municipality|CDP|corporation|"
    r"urban county|County)$"
)

# City-county governments the Census names by both; people write the city.
CONSOLIDATED = {
    "Nashville-Davidson": "Nashville", "Lexington-Fayette": "Lexington",
    "Augusta-Richmond": "Augusta", "Macon-Bibb": "Macon", "Athens-Clarke": "Athens",
    "Butte-Silver Bow": "Butte", "Anaconda-Deer Lodge": "Anaconda",
    "Cusseta-Chattahoochee": "Cusseta",
}

# Well-known places that aren't incorporated (Census "CDPs"), so they're not
# in the city estimates. Populations are rounded 2020 census counts.
EXTRA = {
    ("Honolulu", "HI"): 350000, ("Arlington", "VA"): 238000, ("The Woodlands", "TX"): 114000,
    ("Columbia", "MD"): 104000, ("Silver Spring", "MD"): 81000, ("Bethesda", "MD"): 68000,
    ("Reston", "VA"): 63000, ("McLean", "VA"): 50000, ("Tysons", "VA"): 26000,
    ("Metairie", "LA"): 143000, ("Paradise", "NV"): 191000, ("Spring", "TX"): 62000,
}

places = dict(EXTRA)
with open(f"{src}/sub-est.csv", encoding="latin-1") as f:
    for r in csv.DictReader(f):
        if r["SUMLEV"] != "162" or r["STNAME"] not in STATES:
            continue
        name = r["NAME"]
        for _ in range(2):
            name = SUFFIX.sub("", name).strip()
        # "Nashville-Davidson" stays; "Louisville/Jefferson County" -> "Louisville"
        name = name.split("/")[0].strip()
        name = CONSOLIDATED.get(name, name)
        if not name:
            continue
        key = (name, STATES[r["STNAME"]])
        pop = int(r["POPESTIMATE2023"] or 0)
        places[key] = max(pop, places.get(key, 0))

with open(f"{out}/us_places.csv", "w", newline="", encoding="utf-8") as f:
    w = csv.writer(f)
    w.writerow(["name", "state", "population"])
    for (name, st), pop in sorted(places.items(), key=lambda kv: (-kv[1], kv[0])):
        w.writerow([name, st, pop])
print("places", len(places))

schools = {}
with open(f"{src}/hd/HD2023.csv", encoding="utf-8-sig", errors="replace") as f:
    for r in csv.DictReader(f):
        if r["CYACTIVE"] != "1" or r["DEGGRANT"] != "1":
            continue
        size = int(r["INSTSIZE"]) if r["INSTSIZE"].lstrip("-").isdigit() else 0
        rank = max(size, 0) * 10 + (5 if r["ICLEVEL"] == "1" else 0)
        alias = " ".join(r["IALIAS"].split())[:200]
        key = (r["INSTNM"].strip(), r["CITY"].strip(), r["STABBR"].strip())
        if key[0]:
            schools[key] = (rank, alias)

with open(f"{out}/us_schools.csv", "w", newline="", encoding="utf-8") as f:
    w = csv.writer(f)
    w.writerow(["name", "city", "state", "alias", "rank"])
    for (name, city, st), (rank, alias) in sorted(schools.items(), key=lambda kv: (-kv[1][0], kv[0])):
        w.writerow([name, city, st, alias, rank])
print("schools", len(schools))

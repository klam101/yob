/**
 * States and countries for type-ahead on address fields. Static lists: no
 * lookup leaves the app.
 */

export const US_STATES: readonly [code: string, name: string][] = [
  ["AL", "Alabama"], ["AK", "Alaska"], ["AZ", "Arizona"], ["AR", "Arkansas"], ["CA", "California"],
  ["CO", "Colorado"], ["CT", "Connecticut"], ["DE", "Delaware"], ["DC", "District of Columbia"],
  ["FL", "Florida"], ["GA", "Georgia"], ["HI", "Hawaii"], ["ID", "Idaho"], ["IL", "Illinois"],
  ["IN", "Indiana"], ["IA", "Iowa"], ["KS", "Kansas"], ["KY", "Kentucky"], ["LA", "Louisiana"],
  ["ME", "Maine"], ["MD", "Maryland"], ["MA", "Massachusetts"], ["MI", "Michigan"], ["MN", "Minnesota"],
  ["MS", "Mississippi"], ["MO", "Missouri"], ["MT", "Montana"], ["NE", "Nebraska"], ["NV", "Nevada"],
  ["NH", "New Hampshire"], ["NJ", "New Jersey"], ["NM", "New Mexico"], ["NY", "New York"],
  ["NC", "North Carolina"], ["ND", "North Dakota"], ["OH", "Ohio"], ["OK", "Oklahoma"], ["OR", "Oregon"],
  ["PA", "Pennsylvania"], ["RI", "Rhode Island"], ["SC", "South Carolina"], ["SD", "South Dakota"],
  ["TN", "Tennessee"], ["TX", "Texas"], ["UT", "Utah"], ["VT", "Vermont"], ["VA", "Virginia"],
  ["WA", "Washington"], ["WV", "West Virginia"], ["WI", "Wisconsin"], ["WY", "Wyoming"],
  ["PR", "Puerto Rico"], ["GU", "Guam"], ["VI", "U.S. Virgin Islands"],
];

/** "South Carolina (SC)": shown in the menu; picking stores the code. */
export const STATE_OPTIONS = US_STATES.map(([code, name]) => `${name} (${code})`);

/** "South Carolina (SC)" -> "SC". */
export const stateCode = (option: string) => /\(([A-Z]{2})\)$/.exec(option)?.[1] ?? option;

/** Whether a country value means the US (empty counts: the app is US-first). */
export const isUs = (country: string) =>
  !country.trim() || /^(us|usa|u\.s\.a?\.?|united states( of america)?|america)$/i.test(country.trim());

/**
 * Countries by their common English name (ISO 3166-1), United States and
 * other frequent ones first; the rest alphabetical. Codes and other names
 * are in `COUNTRY_ALIASES` for matching ("uk", "usa").
 */
export const COUNTRIES: readonly string[] = [
  "United States", "Canada", "United Kingdom", "India", "Mexico", "Germany", "China", "Philippines",
  "Australia", "Brazil", "France", "Japan", "South Korea", "Vietnam", "Nigeria", "Ireland",
  "Afghanistan", "Albania", "Algeria", "Andorra", "Angola", "Antigua and Barbuda", "Argentina", "Armenia",
  "Austria", "Azerbaijan", "Bahamas", "Bahrain", "Bangladesh", "Barbados", "Belarus", "Belgium", "Belize",
  "Benin", "Bhutan", "Bolivia", "Bosnia and Herzegovina", "Botswana", "Brunei", "Bulgaria", "Burkina Faso",
  "Burundi", "Cabo Verde", "Cambodia", "Cameroon", "Central African Republic", "Chad", "Chile", "Colombia",
  "Comoros", "Congo", "Costa Rica", "Côte d'Ivoire", "Croatia", "Cuba", "Cyprus", "Czechia",
  "Democratic Republic of the Congo", "Denmark", "Djibouti", "Dominica", "Dominican Republic", "Ecuador",
  "Egypt", "El Salvador", "Equatorial Guinea", "Eritrea", "Estonia", "Eswatini", "Ethiopia", "Fiji",
  "Finland", "Gabon", "Gambia", "Georgia", "Ghana", "Greece", "Grenada", "Guatemala", "Guinea",
  "Guinea-Bissau", "Guyana", "Haiti", "Honduras", "Hong Kong", "Hungary", "Iceland", "Indonesia", "Iran",
  "Iraq", "Israel", "Italy", "Jamaica", "Jordan", "Kazakhstan", "Kenya", "Kiribati", "Kosovo", "Kuwait",
  "Kyrgyzstan", "Laos", "Latvia", "Lebanon", "Lesotho", "Liberia", "Libya", "Liechtenstein", "Lithuania",
  "Luxembourg", "Macao", "Madagascar", "Malawi", "Malaysia", "Maldives", "Mali", "Malta", "Marshall Islands",
  "Mauritania", "Mauritius", "Micronesia", "Moldova", "Monaco", "Mongolia", "Montenegro", "Morocco",
  "Mozambique", "Myanmar", "Namibia", "Nauru", "Nepal", "Netherlands", "New Zealand", "Nicaragua", "Niger",
  "North Korea", "North Macedonia", "Norway", "Oman", "Pakistan", "Palau", "Palestine", "Panama",
  "Papua New Guinea", "Paraguay", "Peru", "Poland", "Portugal", "Puerto Rico", "Qatar", "Romania", "Russia",
  "Rwanda", "Saint Kitts and Nevis", "Saint Lucia", "Saint Vincent and the Grenadines", "Samoa",
  "San Marino", "São Tomé and Príncipe", "Saudi Arabia", "Senegal", "Serbia", "Seychelles", "Sierra Leone",
  "Singapore", "Slovakia", "Slovenia", "Solomon Islands", "Somalia", "South Africa", "South Sudan", "Spain",
  "Sri Lanka", "Sudan", "Suriname", "Sweden", "Switzerland", "Syria", "Taiwan", "Tajikistan", "Tanzania",
  "Thailand", "Timor-Leste", "Togo", "Tonga", "Trinidad and Tobago", "Tunisia", "Turkey", "Turkmenistan",
  "Tuvalu", "Uganda", "Ukraine", "United Arab Emirates", "Uruguay", "Uzbekistan", "Vanuatu", "Vatican City",
  "Venezuela", "Yemen", "Zambia", "Zimbabwe",
];

/** Extra words a country is found by: its codes and other common names. */
export const COUNTRY_ALIASES: Record<string, string> = {
  "United States": "us usa america united states of america",
  "United Kingdom": "uk gb great britain england scotland wales britain",
  "United Arab Emirates": "uae ae emirates",
  "South Korea": "kr korea republic of korea",
  "North Korea": "kp dprk",
  "Czechia": "cz czech republic",
  "Netherlands": "nl holland",
  "Russia": "ru russian federation",
  "China": "cn prc",
  "India": "in",
  "Canada": "ca",
  "Mexico": "mx",
  "Germany": "de deutschland",
  "Philippines": "ph",
  "Australia": "au",
  "Brazil": "br brasil",
  "France": "fr",
  "Japan": "jp",
  "Vietnam": "vn viet nam",
  "Ireland": "ie",
  "New Zealand": "nz",
  "Côte d'Ivoire": "ivory coast ci",
  "Eswatini": "swaziland",
  "Myanmar": "burma",
  "Cabo Verde": "cape verde",
  "Timor-Leste": "east timor",
  "Turkey": "türkiye turkiye",
  "Democratic Republic of the Congo": "drc congo kinshasa",
  "Congo": "republic of the congo brazzaville",
  "Taiwan": "tw",
  "Hong Kong": "hk",
  "Singapore": "sg",
  "Spain": "es españa",
  "Italy": "it italia",
  "Poland": "pl",
  "Israel": "il",
  "Pakistan": "pk",
  "Nigeria": "ng",
  "South Africa": "za",
};

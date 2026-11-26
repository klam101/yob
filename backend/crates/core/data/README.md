# Reference data for type-ahead suggestions

Fixed lists that `core::suggest` searches in memory (compiled in with
`include_str!`). Both come from US government data, which is public domain.

| File | What | Source |
| --- | --- | --- |
| `us_places.csv` | About 19.5k US cities and towns as name, state and population; a few well-known unincorporated places (Honolulu, Arlington VA, …) added by hand | U.S. Census Bureau, city and town population estimates (`sub-est2023.csv`, incorporated places) |
| `us_schools.csv` | About 4.1k active degree-granting colleges and universities: name, city, state, alias ("UCLA"), and a rank from enrollment size and 4-year status | NCES IPEDS institutional characteristics directory (`HD2023.csv`) |

To refresh them, download the two source files into a folder (unzip
`HD2023.zip` into `hd/` there) and run:

```sh
python -I backend/crates/core/data/build.py <download folder> backend/crates/core/data
```

Job titles and company names aren't here: the worker builds them from the
job data (`suggest_terms` table).

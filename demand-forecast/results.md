# Demand backtest — NYC taxi (Jan 2024, last 7 days held out)

62 zones · hourly · lower MAE/RMSE/wMAPE = better.

| Model | MAE | RMSE | wMAPE | bias |
|---|---|---|---|---|
| LastWeek (lag 168h) | 11.261 | 21.178 | 0.1695 | -1.677 |
| SeasonalNaive (zone x dow x hod) | 12.725 | 23.779 | 0.1915 | -4.33 |
| ZoneMean | 41.021 | 65.968 | 0.6174 | -4.734 |
| GlobalMean | 62.687 | 86.3 | 0.9435 | -4.734 |

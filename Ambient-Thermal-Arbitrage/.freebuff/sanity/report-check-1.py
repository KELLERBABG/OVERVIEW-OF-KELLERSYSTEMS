cases = [
 ('schwach',10,1,100,20,.10,1000),
 ('konservativ',10,2,150,40,.05,1000),
 ('guenstig',20,2,200,60,.03,1000),
 ('upgrade',30,2.5,200,60,.03,1500),
 ('tk_lager',100,3,200,60,.05,3000),
]
for name,p,h,n,spread,extra,om in cases:
    e = p*h*n/1000
    gross = e*spread - e*extra*150
    print(name, 'MWh',e,'vor_OM',gross,'nach_OM',gross-om,
          'Kunde_50pct_Fee',gross*.5-om)
# Kombiniertes Paket: keine Reserve, alle drei Ziele gleichzeitig vorausgesetzt.
g = 832.5 + (1000*289*.52/1000)*.05*200 + 15*150
assert abs(g-4585.3) < 1e-8
print('Paket',g,'netto',g-1500,'Jahre',15000/(g-1500))
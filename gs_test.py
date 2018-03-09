from query.gsquery import GoldsrcQuery

con = GoldsrcQuery("77.220.180.87", 27015, timeout=10)
challenge = con.a2s_players_challenge()
print(con.a2s_players(challenge))
print(con.a2s_rules(challenge))

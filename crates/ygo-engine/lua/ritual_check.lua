-- DuelForge: proc_ritual.lua's Ritual.Check with the same answers, for less
-- work. Upstream sums the Levels with Group.GetSum, which enters Lua from
-- the engine once per card for every group the selection loop tries
-- (aux.SelectUnselectGroup): with a dozen materials in the Deck, hundreds
-- of thousands of entries in one request.
--
-- Here the sum is taken in Lua, and each card's Level is asked once per
-- check: nothing changes the duel while a group is being checked.
function Ritual.Check(sc,lv,forcedselection,_type,requirementfunc)
	local levelf=requirementfunc or Auxiliary.RitualCheckAdditionalLevel
	local levels={}
	local function level(c)
		local v=levels[c]
		if v==nil then
			-- what Group.GetSum makes of the value: rounded, 0 if not a number
			v=levelf(c,sc)
			if math.type(v)~="integer" then v=type(v)=="number" and math.floor(v+0.5) or 0 end
			levels[c]=v
		end
		return v
	end
	local function sum(g)
		local s=0
		local c=g:GetFirst()
		while c do
			s=s+level(c)
			c=g:GetNext()
		end
		return s
	end
	local chk
	if _type==RITPROC_EQUAL then
		chk=function(g) return sum(g)<=lv end
	else
		chk=function(g,c) return sum(g) - level(c)<=lv end
	end
	return function(sg,e,tp,mg,c)
		local res=chk(sg,c)
		if not res then return false,true end
		local stop=false
		if forcedselection then
			local ret=forcedselection(e,tp,sg,sc)
			res=ret[1]
			stop=ret[2] or stop
		end
		if res and not stop then
			if _type==RITPROC_EQUAL then
				res=sg:CheckWithSumEqual(requirementfunc or Card.GetRitualLevel,lv,#sg,#sg,sc)
			else
				Duel.SetSelectedCard(sg)
				res=sg:CheckWithSumGreater(requirementfunc or Card.GetRitualLevel,lv,sc)
			end
			res=res and (sc:IsLocation(LOCATION_EXTRA) and Duel.GetLocationCountFromEx(tp,tp,sg,sc) or Duel.GetMZoneCount(tp,sg,tp))>0
		end
		return res,stop
	end
end

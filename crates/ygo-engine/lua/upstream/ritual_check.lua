function Ritual.Check(sc,lv,forcedselection,_type,requirementfunc)
	local chk
	if _type==RITPROC_EQUAL then
		chk=function(g) return g:GetSum(requirementfunc or Auxiliary.RitualCheckAdditionalLevel,sc)<=lv end
	else
		chk=function(g,c) return g:GetSum(requirementfunc or Auxiliary.RitualCheckAdditionalLevel,sc) - (requirementfunc or Auxiliary.RitualCheckAdditionalLevel)(c,sc)<=lv end
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
			local stop=false
			res=res and (sc:IsLocation(LOCATION_EXTRA) and Duel.GetLocationCountFromEx(tp,tp,sg,sc) or Duel.GetMZoneCount(tp,sg,tp))>0
		end
		return res,stop
	end
end

create or replace function get_call_info return varchar as

	name varchar(64) := 'none';
	surname varchar(64) := 'none';
	num varchar(16) := 'none';
BEGIN
	select
		c.nom, c.prenom, c.numero into surname, name, num
	from
		Contact c
	where
		c.to_call = 1 and
		not exists (
			select
				cc.*
			from
				Contact cc
			where
				cc.to_call = 1 and
				cc.last_call <= c.last_call
		);

	return num;
END;
/

create or replace procedure touch_call_info(num IN varchar) as
BEGIN
	update Contact set to_call = 0 where numero = num;
	update Contact set last_call = current_date where numero = num;
END;
/
## Calculate if input % 3 is 0, 1, or 2
q <- sample(seq(1,10000), 10000)
data <- cbind(q, q%%3==0, q%%3==1, q%%3==2)
colnames(data) <- c("Q", "Zero", "One", "Two")
data <- rbind(c(1,0,0,0), data)
write.csv(data, "data.in", row.names=FALSE, quote=FALSE)

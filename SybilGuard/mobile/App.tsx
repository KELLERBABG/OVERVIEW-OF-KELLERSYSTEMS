import React from 'react';
import { View, Text, StyleSheet } from 'react-native';

export default function App() {
  return (
    <View style={styles.container}>
      <Text style={styles.title}>Sybil-Resistentes</Text>
      <Text style={styles.subtitle}>Proof-of-Personhood</Text>
      <View style={styles.status}>
        <Text style={styles.statusText}>Wallet not connected</Text>
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#fff',
    alignItems: 'center',
    justifyContent: 'center',
    padding: 20,
  },
  title: {
    fontSize: 28,
    fontWeight: 'bold',
    marginBottom: 8,
  },
  subtitle: {
    fontSize: 16,
    color: '#666',
    marginBottom: 40,
  },
  status: {
    padding: 16,
    backgroundColor: '#f0f0f0',
    borderRadius: 8,
  },
  statusText: {
    color: '#888',
  },
});
package pt.adrz.clipx.gui.list;

import java.util.LinkedList;

import javax.swing.AbstractListModel;
import javax.swing.JTextField;

public class ClipListModel<E> extends AbstractListModel<E> {
	private static final long serialVersionUID = 1L;

	private LinkedList<E> items;
	private LinkedList<E> filteredItems;
	private JTextField search;
	
	public ClipListModel() {
		items = new LinkedList<E>();
		filteredItems = new LinkedList<E>();
	}
	
	public ClipListModel(JTextField search) {
		this();
		this.search = search;
	}

	public LinkedList<E> getItems() {
		return items;
	}

	public void setItems(LinkedList<E> items) {
		this.items = items;
		this.refilter();
	}
	
	public void addElement(E element) {
		items.add(element);
		this.refilter();
	}
	
	public void addElementTo(E element, int index) {
		items.add(index,element);
		this.refilter();
	}
	
	public void switchVals(int index, E element) {
		items.remove(index);
		items.add(0, element);
		this.refilter();
	}
	
	public void refilter() {

		filteredItems.clear();

		String term = this.search.getText();

		for (int i = 0 ; i < items.size() ; i++) 
			if (items.get(i).toString().toLowerCase().indexOf(term.toLowerCase(),0)!=-1)
				filteredItems.add(items.get(i));

		fireContentsChanged(this, 0, getSize());
	}

	@Override
	public int getSize() {
		return filteredItems.size();
	}

	@Override
	public E getElementAt(int index) {

		if ( index == -1)
			return null;
		
		if (index < filteredItems.size())
            return filteredItems.get (index);

		return null;
	}

	public void remove(int index) {
		items.remove(index);
		this.refilter();
	}
}
